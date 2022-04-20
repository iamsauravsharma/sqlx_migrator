use sqlx::{Error, Transaction};

type OperationFunction<D> = Box<dyn Fn(&mut Transaction<D>) -> Result<(), Error>>;

/// Struct to store up operation and corresponding down operation
pub struct Operation<D>
where
    D: sqlx::Database,
{
    up_function: OperationFunction<D>,
    down_function: OperationFunction<D>,
}

impl<D> Operation<D>
where
    D: sqlx::Database,
{
    /// Create new operation from up function and down function
    pub fn new(up_function: OperationFunction<D>, down_function: OperationFunction<D>) -> Self {
        Self {
            up_function,
            down_function,
        }
    }

    /// Return up function
    pub fn up(&self) -> &OperationFunction<D> {
        &self.up_function
    }

    /// Return down function
    pub fn down(&self) -> &OperationFunction<D> {
        &self.down_function
    }

    /// Apply operation
    pub fn apply(&self, transaction: &mut Transaction<D>) -> Result<(), Error> {
        self.up()(transaction)
    }

    /// Revert operation
    pub fn revert(&self, transaction: &mut Transaction<D>) -> Result<(), Error> {
        self.down()(transaction)
    }
}

impl<D> From<(OperationFunction<D>, OperationFunction<D>)> for Operation<D>
where
    D: sqlx::Database,
{
    fn from((up_function, down_function): (OperationFunction<D>, OperationFunction<D>)) -> Self {
        Self {
            up_function,
            down_function,
        }
    }
}
