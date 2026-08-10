pub struct FocusCursor<T: Copy + Eq + 'static> {
    items: &'static [T],
    index: usize,
}

impl<T: Copy + Eq + 'static> FocusCursor<T> {
    pub const fn new(items: &'static [T]) -> Self {
        assert!(
            !items.is_empty(),
            "FocusCursor requires a non-empty item list"
        );
        Self { items, index: 0 }
    }

    pub fn current(&self) -> T {
        self.items[self.index]
    }

    pub fn is(&self, t: T) -> bool {
        self.current() == t
    }

    pub fn next(&mut self) -> T {
        self.index = (self.index + 1) % self.items.len();
        self.current()
    }

    pub fn prev(&mut self) -> T {
        self.index = (self.index + self.items.len() - 1) % self.items.len();
        self.current()
    }
}
