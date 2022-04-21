use crate::Operation;

/// Trait for migration
pub trait Migration {
    /// Migration name
    fn name(&self) -> String;

    /// Parents of migration
    fn parents(&self) -> Vec<Box<dyn Migration>>;

    /// Operation performed for migration
    fn operations(&self) -> Vec<Box<dyn Operation>>;
}
