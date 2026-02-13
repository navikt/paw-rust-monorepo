pub fn contains_all<T: PartialEq>(collection: &[T], expected: &[T]) -> bool {
    expected.iter().all(|item| collection.contains(item))
}
