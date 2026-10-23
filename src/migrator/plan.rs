//! [`Plan`] describing which migrations a run applies or reverts

#[derive(Debug, Default)]
pub(super) struct RevertPlan {
    pub(super) last_batch: bool,
}

#[derive(Debug)]
pub(super) enum PlanType {
    Apply,
    Revert(RevertPlan),
}

/// Struct that determines the type of migration plan to execute.
///
/// A [`Plan`] can specify whether to apply or revert migrations, and may target
/// all migrations, specific migrations, or a limited number of migrations.
#[derive(Debug)]
pub struct Plan {
    #[expect(
        clippy::struct_field_names,
        reason = "type is a keyword so it cannot be used"
    )]
    pub(super) plan_type: PlanType,
    pub(super) app_migration: Option<(String, Option<String>)>,
    pub(super) count: Option<usize>,
    pub(super) fake: bool,
}

impl Plan {
    fn new(
        plan_type: PlanType,
        app_migration: Option<(String, Option<String>)>,
        count: Option<usize>,
    ) -> Self {
        Self {
            plan_type,
            app_migration,
            count,
            fake: false,
        }
    }

    /// Sets the plan as a "fake" plan.
    ///
    /// When the plan is marked as fake, the migration status is updated to
    /// either "applied" or "reverted" without actually performing any
    /// migration operations. This is useful for scenarios where you want to
    /// simulate the effect of applying or reverting a migration, but
    /// without making changes to the database.
    ///
    /// By default, the `fake` flag is set to `false`, and the migration
    /// operations are executed as expected.
    #[must_use]
    pub fn fake(mut self, fake: bool) -> Self {
        self.fake = fake;
        self
    }

    /// Creates a new plan to apply all migrations.
    #[must_use]
    pub fn apply_all() -> Self {
        Self::new(PlanType::Apply, None, None)
    }

    /// Creates a new plan to apply a specific migration by name. If migration
    /// name is not provided it will apply all migrations of the app
    #[must_use]
    pub fn apply_name(app: &str, name: Option<&str>) -> Self {
        Self::new(
            PlanType::Apply,
            Some((app.to_string(), name.map(ToString::to_string))),
            None,
        )
    }

    /// Creates a new plan to apply a limited number of migrations.
    #[must_use]
    pub fn apply_count(count: usize) -> Self {
        Self::new(PlanType::Apply, None, Some(count))
    }

    /// Creates a new plan to revert all migrations.
    #[must_use]
    pub fn revert_all() -> Self {
        Self::new(PlanType::Revert(RevertPlan::default()), None, None)
    }

    /// Creates a new plan to revert a specific migration by name. If migration
    /// name is not provided it will revert all migrations of the app
    #[must_use]
    pub fn revert_name(app: &str, name: Option<&str>) -> Self {
        Self::new(
            PlanType::Revert(RevertPlan::default()),
            Some((app.to_string(), name.map(ToString::to_string))),
            None,
        )
    }

    /// Creates a new plan to revert a limited number of migrations.
    #[must_use]
    pub fn revert_count(count: usize) -> Self {
        Self::new(PlanType::Revert(RevertPlan::default()), None, Some(count))
    }

    /// Creates a new plan to revert the migrations applied by the last run,
    /// which share the highest batch number
    #[must_use]
    pub fn revert_last_batch() -> Self {
        let plan_type = PlanType::Revert(RevertPlan { last_batch: true });
        Self::new(plan_type, None, None)
    }
}
