use crate::Operation;

/// Trait for migration
pub trait Migration {
    /// App name for migration
    fn app(&self) -> &str;

    /// Migration name
    fn name(&self) -> &str;

    /// Description of migration
    fn description(&self) -> &str;

    /// Parents of migration
    fn parents(&self) -> Vec<Box<dyn Migration>>;

    /// Operation performed for migration
    fn operations(&self) -> Vec<Box<dyn Operation>>;

    /// Full name of migration created using app and name function by default in
    /// format {app}/{name}
    fn full_name(&self) -> String {
        format!("{}/{}", self.app(), self.name())
    }
}