//! Bounded round-robin selection policy. Ownership, context and hardware remain caller responsibilities.
pub fn next_ready(current: Option<usize>, ready: &[bool]) -> Option<usize> {
    if ready.is_empty() || current.is_some_and(|index| index >= ready.len()) {
        return None;
    }
    let start = current.map_or(0, |index| (index + 1) % ready.len());
    (0..ready.len())
        .map(|step| (start + step) % ready.len())
        .find(|&index| ready[index])
}
#[cfg(test)]
mod tests {
    use super::next_ready;
    #[test]
    fn each_ready_task_is_served_in_one_rotation() {
        let ready = [true, false, true, true, false];
        let mut current = None;
        let mut visited = ready.map(|_| false);
        for _ in 0..ready.iter().filter(|&&r| r).count() {
            let next = next_ready(current, &ready).unwrap();
            assert!(!visited[next]);
            visited[next] = true;
            current = Some(next);
        }
        assert_eq!(visited, ready);
        assert_eq!(next_ready(current, &ready), Some(0));
    }
    #[test]
    fn empty_exhausted_and_invalid_queues_have_no_selection() {
        assert_eq!(next_ready(None, &[]), None);
        assert_eq!(next_ready(Some(0), &[false]), None);
        assert_eq!(next_ready(Some(usize::MAX), &[true]), None);
    }
}
