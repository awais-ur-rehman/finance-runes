//! Categories: the labels that explain why money moved.

use crate::CategoryId;

/// Whether a category describes money leaving or money arriving.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CategoryKind {
    /// Spending, such as groceries or rent.
    Expense,
    /// Income, such as salary.
    Income,
}

impl std::fmt::Display for CategoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Expense => "expense",
            Self::Income => "income",
        };
        f.write_str(name)
    }
}

/// A category, optionally nested under a parent.
///
/// The hierarchy is limited to two levels: a parent with children, where the
/// parent may also be used directly (FR-040).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Category {
    id: CategoryId,
    name: String,
    kind: CategoryKind,
    parent: Option<CategoryId>,
}

impl Category {
    /// Creates a top-level category.
    pub fn new(id: CategoryId, name: impl Into<String>, kind: CategoryKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            parent: None,
        }
    }

    /// Nests this category under a parent.
    #[must_use]
    pub fn with_parent(mut self, parent: CategoryId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// The category's identifier.
    pub const fn id(&self) -> CategoryId {
        self.id
    }

    /// The category's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether this category describes spending or income.
    pub const fn kind(&self) -> CategoryKind {
        self.kind
    }

    /// The parent category, if this is a child.
    pub const fn parent(&self) -> Option<CategoryId> {
        self.parent
    }

    /// Renames the category.
    pub fn rename(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
}
