//! Pure dependency solving and a separate checker for untrusted solver output.
use crate::{Package, Requirements, covers, validate_catalog, validate_requirements};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub packages: Vec<String>,
}

/// A bounded host profile, not a KOLVRT coinstallation law.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionModel {
    ReferenceSingleVersion,
    SideBySideNamespaces,
}

pub const MAX_SCOPED_NAMESPACES: usize = 64;

/// One independently resolved process/runtime namespace with explicit target ABI and retained
/// consumer requirements. Namespace IDs must be sorted and unique for deterministic plans.
pub struct ScopedNamespace<'a> {
    pub id: &'a str,
    pub root: &'a str,
    pub architecture: &'a str,
    pub abi: &'a str,
    pub requirements: &'a Requirements,
    pub retained: &'a Requirements,
}

pub struct ScopedProblem<'a> {
    pub catalog: &'a [Package],
    pub namespaces: &'a [ScopedNamespace<'a>],
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ScopedSelection {
    pub namespace: String,
    pub identity: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScopedPlan {
    pub selections: Vec<ScopedSelection>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScopedSearchMetrics {
    /// Unique selected-package states visited by the bounded search.
    pub visited_states: u64,
    /// Feasible complete closures reached before optimality pruning.
    pub feasible_closures: u64,
    /// Candidate package additions considered across the visited states.
    pub branches_considered: u64,
}

pub struct Problem<'a> {
    pub catalog: &'a [Package],
    pub requirements: &'a Requirements,
    pub root: &'a str,
    /// All retained consumers' requirements; supplied explicitly by the caller.
    pub retained: &'a Requirements,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SolveError {
    Invalid(String),
    Unsatisfiable,
    BudgetExceeded,
}

pub trait Solver {
    fn solve(&self, problem: &Problem<'_>) -> Result<Plan, SolveError>;
}

fn validate_problem(problem: &Problem<'_>) -> Result<(), String> {
    validate_catalog(problem.catalog)?;
    validate_requirements(problem.requirements)?;
    validate_requirements(problem.retained)?;
    let root = problem
        .catalog
        .iter()
        .find(|p| p.identity == problem.root)
        .ok_or("unknown root identity")?;
    if problem.requirements.is_empty() || !covers(&[root], problem.requirements) {
        return Err("root does not provide the requested replacement surface".into());
    }
    Ok(())
}

/// Checks the reference single-version profile without trusting the solver's search.
/// Side-by-side ABI/runtime/sandbox namespaces need a different explicit model/checker.
pub fn verify(problem: &Problem<'_>, plan: &Plan) -> Result<(), String> {
    validate_problem(problem)?;
    if plan.packages.is_empty() || plan.packages.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("plan identities must be unique and sorted".into());
    }
    let selected: Vec<_> = plan
        .packages
        .iter()
        .map(|identity| {
            problem
                .catalog
                .iter()
                .find(|p| &p.identity == identity)
                .ok_or("unknown plan identity")
        })
        .collect::<Result<_, _>>()?;
    if !plan.packages.iter().any(|p| p == problem.root) {
        return Err("missing root".into());
    }
    let names: BTreeSet<_> = selected.iter().map(|p| &p.name).collect();
    if names.len() != selected.len() {
        return Err("reference profile does not model side-by-side version namespaces".into());
    }
    if !covers(&selected, problem.retained) {
        return Err("retained consumer requirement is unsatisfied".into());
    }
    for p in &selected {
        if !covers(&selected, &p.requires) {
            return Err(format!("unsatisfied dependency: {}", p.name));
        }
        if p.conflicts.iter().any(|r| {
            selected.iter().any(|other| {
                other.identity != p.identity && other.provides.iter().any(|c| r.matches(c))
            })
        }) {
            return Err(format!("conflict: {}", p.name));
        }
    }
    Ok(())
}

/// Small independent oracle: complete subset enumeration; minimizes package count, then identities.
/// This is not a SAT backend or a CUDF implementation. Exhausted search is never UNSAT.
pub struct ExhaustiveSolver {
    pub max_states: u64,
}
impl Solver for ExhaustiveSolver {
    fn solve(&self, problem: &Problem<'_>) -> Result<Plan, SolveError> {
        validate_problem(problem).map_err(SolveError::Invalid)?;
        if problem.catalog.len() > crate::MAX_REFERENCE_PACKAGES {
            return Err(SolveError::Invalid(format!(
                "exhaustive reference solver is limited to {} packages",
                crate::MAX_REFERENCE_PACKAGES
            )));
        }
        let root_index = problem
            .catalog
            .iter()
            .position(|p| p.identity == problem.root)
            .unwrap();
        let total = 1u64 << problem.catalog.len();
        let mut states = 0;
        let mut best: Option<Plan> = None;
        for mask in 0..total {
            if mask & (1 << root_index) == 0 {
                continue;
            }
            if states == self.max_states {
                return Err(SolveError::BudgetExceeded);
            }
            states += 1;
            let mut packages: Vec<_> = problem
                .catalog
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, p)| p.identity.clone())
                .collect();
            packages.sort();
            let plan = Plan { packages };
            if best.as_ref().is_some_and(|b| {
                (b.packages.len(), &b.packages) <= (plan.packages.len(), &plan.packages)
            }) {
                continue;
            }
            if verify(problem, &plan).is_ok() {
                best = Some(plan);
            }
        }
        best.ok_or(SolveError::Unsatisfiable)
    }
}

/// Bounded dependency-closure search for larger catalogs. It branches only on providers of
/// currently unsatisfied requirement clauses, memoizes selected closures, and independently
/// verifies each complete plan. This remains the reference single-version profile.
pub struct ClosureSolver {
    pub max_states: u64,
}

fn package_satisfies(package: &Package, clause: &[crate::Requirement]) -> bool {
    clause
        .iter()
        .any(|requirement| package.provides.iter().any(|p| requirement.matches(p)))
}

fn can_add(catalog: &[Package], selected: &[usize], candidate: usize) -> bool {
    let package = &catalog[candidate];
    selected.iter().all(|&index| {
        let other = &catalog[index];
        package.name != other.name
            && !package
                .conflicts
                .iter()
                .any(|r| other.provides.iter().any(|p| r.matches(p)))
            && !other
                .conflicts
                .iter()
                .any(|r| package.provides.iter().any(|p| r.matches(p)))
    })
}

impl Solver for ClosureSolver {
    fn solve(&self, problem: &Problem<'_>) -> Result<Plan, SolveError> {
        validate_problem(problem).map_err(SolveError::Invalid)?;
        let root_index = problem
            .catalog
            .iter()
            .position(|p| p.identity == problem.root)
            .unwrap();
        let mut pending = vec![vec![root_index]];
        let mut visited = BTreeSet::new();
        let mut states = 0;
        let mut best: Option<Plan> = None;

        while let Some(selected) = pending.pop() {
            if !visited.insert(selected.clone()) {
                continue;
            }
            if states >= self.max_states {
                return Err(SolveError::BudgetExceeded);
            }
            states += 1;
            if best
                .as_ref()
                .is_some_and(|plan| selected.len() > plan.packages.len())
            {
                continue;
            }

            let mut clauses: Vec<&Vec<crate::Requirement>> = Vec::new();
            clauses.extend(problem.requirements.iter());
            clauses.extend(problem.retained.iter());
            for &index in &selected {
                clauses.extend(problem.catalog[index].requires.iter());
            }

            let mut next: Option<Vec<usize>> = None;
            let mut impossible = false;
            for clause in clauses {
                if selected
                    .iter()
                    .any(|&i| package_satisfies(&problem.catalog[i], clause))
                {
                    continue;
                }
                let mut providers: Vec<_> = problem
                    .catalog
                    .iter()
                    .enumerate()
                    .filter(|(i, package)| {
                        package_satisfies(package, clause)
                            && can_add(problem.catalog, &selected, *i)
                    })
                    .map(|(i, _)| i)
                    .collect();
                providers.sort_by(|&a, &b| {
                    problem.catalog[a]
                        .identity
                        .cmp(&problem.catalog[b].identity)
                });
                if providers.is_empty() {
                    impossible = true;
                    break;
                }
                if next
                    .as_ref()
                    .is_none_or(|current| providers.len() < current.len())
                {
                    next = Some(providers);
                }
            }
            if impossible {
                continue;
            }

            let Some(providers) = next else {
                let mut packages: Vec<_> = selected
                    .iter()
                    .map(|&i| problem.catalog[i].identity.clone())
                    .collect();
                packages.sort();
                let plan = Plan { packages };
                if verify(problem, &plan).is_ok()
                    && best.as_ref().is_none_or(|current| {
                        (plan.packages.len(), &plan.packages)
                            < (current.packages.len(), &current.packages)
                    })
                {
                    best = Some(plan);
                }
                continue;
            };
            if best
                .as_ref()
                .is_some_and(|plan| selected.len() >= plan.packages.len())
            {
                continue;
            }

            // Reverse push order makes the deterministic identity order the DFS visit order.
            for provider in providers.into_iter().rev() {
                let mut child = selected.clone();
                let at = child.binary_search(&provider).unwrap_err();
                child.insert(at, provider);
                pending.push(child);
            }
        }
        best.ok_or(SolveError::Unsatisfiable)
    }
}

fn matches_namespace(package: &Package, namespace: &ScopedNamespace<'_>) -> bool {
    package.variant.as_ref().is_some_and(|variant| {
        variant.architecture == namespace.architecture && variant.abi == namespace.abi
    })
}

pub fn validate_scoped_problem(problem: &ScopedProblem<'_>) -> Result<(), String> {
    validate_catalog(problem.catalog)?;
    if problem.namespaces.is_empty()
        || problem.namespaces.len() > MAX_SCOPED_NAMESPACES
        || problem
            .namespaces
            .windows(2)
            .any(|pair| pair[0].id >= pair[1].id)
    {
        return Err("scoped namespaces must be nonempty, bounded, sorted, and unique".into());
    }
    for namespace in problem.namespaces {
        if namespace.id.trim().is_empty()
            || namespace.architecture.trim().is_empty()
            || namespace.abi.trim().is_empty()
        {
            return Err("scoped namespace identity and target ABI are required".into());
        }
        validate_requirements(namespace.requirements)?;
        validate_requirements(namespace.retained)?;
        let root = problem
            .catalog
            .iter()
            .find(|package| package.identity == namespace.root)
            .ok_or("unknown scoped root identity")?;
        if !matches_namespace(root, namespace)
            || namespace.requirements.is_empty()
            || !covers(&[root], namespace.requirements)
        {
            return Err(format!(
                "root does not provide the requested surface for namespace {}",
                namespace.id
            ));
        }
    }
    Ok(())
}

/// Independently verifies a plan with one package version per name *inside each namespace*.
/// The same package name, including different ABI/build variants, can coexist across namespaces.
pub fn verify_scoped(problem: &ScopedProblem<'_>, plan: &ScopedPlan) -> Result<(), String> {
    validate_scoped_problem(problem)?;
    if plan.selections.is_empty() || plan.selections.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("scoped selections must be unique and sorted by namespace and identity".into());
    }
    let mut selected = vec![Vec::<&Package>::new(); problem.namespaces.len()];
    for item in &plan.selections {
        let scope_index = problem
            .namespaces
            .binary_search_by(|namespace| namespace.id.cmp(item.namespace.as_str()))
            .map_err(|_| "unknown namespace in scoped plan")?;
        let namespace = &problem.namespaces[scope_index];
        let package = problem
            .catalog
            .iter()
            .find(|package| package.identity == item.identity)
            .ok_or("unknown identity in scoped plan")?;
        if !matches_namespace(package, namespace) {
            return Err("package architecture/ABI does not match its namespace".into());
        }
        if selected[scope_index]
            .iter()
            .any(|other| other.name == package.name)
        {
            return Err(format!(
                "namespace {} selects multiple variants of package {}",
                namespace.id, package.name
            ));
        }
        selected[scope_index].push(package);
    }
    for (index, namespace) in problem.namespaces.iter().enumerate() {
        let packages = &selected[index];
        if !packages
            .iter()
            .any(|package| package.identity == namespace.root)
        {
            return Err(format!("missing root in namespace {}", namespace.id));
        }
        if !covers(packages, namespace.requirements) || !covers(packages, namespace.retained) {
            return Err(format!(
                "unsatisfied consumer in namespace {}",
                namespace.id
            ));
        }
        for package in packages {
            if !covers(packages, &package.requires) {
                return Err(format!(
                    "unsatisfied dependency {} in namespace {}",
                    package.name, namespace.id
                ));
            }
            if package.conflicts.iter().any(|requirement| {
                packages.iter().any(|other| {
                    other.identity != package.identity
                        && other
                            .provides
                            .iter()
                            .any(|provided| requirement.matches(provided))
                })
            }) {
                return Err(format!(
                    "conflict for {} in namespace {}",
                    package.name, namespace.id
                ));
            }
        }
    }
    Ok(())
}

fn can_add_scoped(
    catalog: &[Package],
    selected: &[(usize, usize)],
    scope: usize,
    candidate: usize,
) -> bool {
    let package = &catalog[candidate];
    selected
        .iter()
        .filter(|(at_scope, _)| *at_scope == scope)
        .all(|(_, index)| {
            let other = &catalog[*index];
            package.name != other.name
                && !package.conflicts.iter().any(|requirement| {
                    other
                        .provides
                        .iter()
                        .any(|provided| requirement.matches(provided))
                })
                && !other.conflicts.iter().any(|requirement| {
                    package
                        .provides
                        .iter()
                        .any(|provided| requirement.matches(provided))
                })
        })
}

/// Bounded dependency-closure search over multiple ABI-isolated namespaces. Dependencies and
/// conflicts are evaluated only within their namespace; variants must match its architecture/ABI.
pub struct ScopedClosureSolver {
    pub max_states: u64,
}

impl ScopedClosureSolver {
    pub fn solve(&self, problem: &ScopedProblem<'_>) -> Result<ScopedPlan, SolveError> {
        self.solve_with_metrics(problem).map(|(plan, _)| plan)
    }

    pub fn solve_with_metrics(
        &self,
        problem: &ScopedProblem<'_>,
    ) -> Result<(ScopedPlan, ScopedSearchMetrics), SolveError> {
        validate_scoped_problem(problem).map_err(SolveError::Invalid)?;
        let mut roots = Vec::with_capacity(problem.namespaces.len());
        for (scope, namespace) in problem.namespaces.iter().enumerate() {
            let package = problem
                .catalog
                .iter()
                .position(|package| package.identity == namespace.root)
                .unwrap();
            roots.push((scope, package));
        }
        let mut pending = vec![roots];
        let mut visited = BTreeSet::new();
        let mut metrics = ScopedSearchMetrics::default();
        let mut best: Option<ScopedPlan> = None;
        while let Some(selected) = pending.pop() {
            if !visited.insert(selected.clone()) {
                continue;
            }
            if metrics.visited_states >= self.max_states {
                return Err(SolveError::BudgetExceeded);
            }
            metrics.visited_states += 1;
            if best
                .as_ref()
                .is_some_and(|plan| selected.len() > plan.selections.len())
            {
                continue;
            }

            let mut clauses = Vec::<(usize, &Requirements)>::new();
            for (scope, namespace) in problem.namespaces.iter().enumerate() {
                clauses.push((scope, namespace.requirements));
                clauses.push((scope, namespace.retained));
            }
            for &(scope, package) in &selected {
                clauses.push((scope, &problem.catalog[package].requires));
            }

            let mut next: Option<Vec<(usize, usize)>> = None;
            let mut impossible = false;
            for (scope, requirements) in clauses {
                for clause in requirements {
                    if selected.iter().any(|(at_scope, index)| {
                        *at_scope == scope && package_satisfies(&problem.catalog[*index], clause)
                    }) {
                        continue;
                    }
                    let mut providers: Vec<_> = problem
                        .catalog
                        .iter()
                        .enumerate()
                        .filter(|(index, package)| {
                            matches_namespace(package, &problem.namespaces[scope])
                                && package_satisfies(package, clause)
                                && can_add_scoped(problem.catalog, &selected, scope, *index)
                        })
                        .map(|(index, _)| (scope, index))
                        .collect();
                    metrics.branches_considered = metrics
                        .branches_considered
                        .checked_add(providers.len() as u64)
                        .ok_or(SolveError::BudgetExceeded)?;
                    providers.sort_by(|(_, a), (_, b)| {
                        problem.catalog[*a]
                            .identity
                            .cmp(&problem.catalog[*b].identity)
                    });
                    if providers.is_empty() {
                        impossible = true;
                        break;
                    }
                    if next
                        .as_ref()
                        .is_none_or(|current| providers.len() < current.len())
                    {
                        next = Some(providers);
                    }
                }
                if impossible {
                    break;
                }
            }
            if impossible {
                continue;
            }
            let Some(providers) = next else {
                let mut selections: Vec<_> = selected
                    .iter()
                    .map(|(scope, package)| ScopedSelection {
                        namespace: problem.namespaces[*scope].id.into(),
                        identity: problem.catalog[*package].identity.clone(),
                    })
                    .collect();
                selections.sort();
                let plan = ScopedPlan { selections };
                if verify_scoped(problem, &plan).is_ok() {
                    metrics.feasible_closures = metrics
                        .feasible_closures
                        .checked_add(1)
                        .ok_or(SolveError::BudgetExceeded)?;
                }
                if verify_scoped(problem, &plan).is_ok()
                    && best.as_ref().is_none_or(|current| {
                        (plan.selections.len(), &plan.selections)
                            < (current.selections.len(), &current.selections)
                    })
                {
                    best = Some(plan);
                }
                continue;
            };
            if best
                .as_ref()
                .is_some_and(|plan| selected.len() >= plan.selections.len())
            {
                continue;
            }
            for provider in providers.into_iter().rev() {
                let mut child = selected.clone();
                let at = child.binary_search(&provider).unwrap_err();
                child.insert(at, provider);
                pending.push(child);
            }
        }
        best.map(|plan| (plan, metrics))
            .ok_or(SolveError::Unsatisfiable)
    }
}
