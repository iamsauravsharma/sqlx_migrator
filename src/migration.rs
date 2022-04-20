use std::hash::{Hash, Hasher};

use sqlx::{Error, Transaction};

use crate::operation::Operation;

/// Struct to store migration
pub struct Migration<'a, D>
where
    D: sqlx::Database,
{
    identifier: &'a str,
    parents: Vec<Migration<'a, D>>,
    operations: Vec<Operation<D>>,
}

impl<'a, D> Migration<'a, D>
where
    D: sqlx::Database,
{
    /// Create new migration with identifier
    pub fn new(identifier: &'a str) -> Self {
        Self {
            identifier,
            parents: vec![],
            operations: vec![],
        }
    }

    /// get identifier of migration
    pub fn identifier(&self) -> &str {
        self.identifier
    }

    /// get parents of migration
    pub fn parents(&self) -> &[Migration<D>] {
        &self.parents
    }

    /// get operation of migration
    pub fn operations(&self) -> &[Operation<D>] {
        &self.operations
    }

    /// Add parent to migration
    pub fn add_parent(&mut self, parent: Migration<'a, D>) {
        self.parents.push(parent);
    }

    /// Add operation to migration
    pub fn add_operation(&mut self, operation: Operation<D>) {
        self.operations.push(operation);
    }

    /// Apply migration operations
    pub fn apply(&self, transaction: &mut Transaction<D>) -> Result<(), Error> {
        for operation in &self.operations {
            operation.apply(transaction)?;
        }
        Ok(())
    }

    /// Revert migration operations
    pub fn revert(&self, transaction: &mut Transaction<D>) -> Result<(), Error> {
        for operation in self.operations.iter().rev() {
            operation.revert(transaction)?;
        }
        Ok(())
    }
}

impl<'a, D> Hash for Migration<'a, D>
where
    D: sqlx::Database,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.identifier.hash(state);
        self.parents.hash(state);
    }
}

impl<'a, D> PartialEq for Migration<'a, D>
where
    D: sqlx::Database,
{
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier && self.parents == other.parents
    }
}

impl<'a, D> Eq for Migration<'a, D> where D: sqlx::Database {}
