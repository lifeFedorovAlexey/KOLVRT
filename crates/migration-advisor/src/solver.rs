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
