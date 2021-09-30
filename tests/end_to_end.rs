use async_trait::async_trait;
use cucumber::{Context, Cucumber, World};
use error::Error;
use mimir2::adapters::secondary::elasticsearch::remote::connection_test_pool;
use std::any::Any;
use std::convert::Infallible;

mod error;
mod steps;
mod utils;

/// Exit status for a step.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum StepStatus {
    Done,
    Skipped,
}

/// A step which can be run from current state.
#[async_trait(?Send)]
pub trait Step: Sized + 'static {
    async fn execute(&mut self, world: &State) -> Result<StepStatus, Error>;
}

/// Register the steps that have been executed so far.
///
/// This acts as a very generic history used to query what steps have been
/// executed before, filtered by kind (using `steps_for`) or exact match (using
/// `status_of`).
#[derive(Default)]
pub struct State(Vec<(Box<dyn Any>, StepStatus)>);

impl State {
    /// Execute a step and update state accordingly.
    pub async fn execute<S: Step>(&mut self, mut step: S) -> Result<StepStatus, Error> {
        let status = step.execute(self).await?;
        self.0.push((Box::new(step), status));
        Ok(status)
    }

    /// Execute a step and update state accordingly if and only if it has not
    /// been executed before.
    pub async fn execute_once<S: Step + PartialEq>(
        &mut self,
        step: S,
    ) -> Result<StepStatus, Error> {
        match self.status_of(&step) {
            Some(status) => Ok(status),
            None => self.execute(step).await,
        }
    }

    /// Check if given step has already been performed according to current state
    /// and return the status of last run.
    pub fn status_of<S: Step + PartialEq>(&self, step: &S) -> Option<StepStatus> {
        self.steps_for::<S>()
            .filter(|(step_from_state, _)| *step_from_state == step)
            .map(|(_, status)| status)
            .next_back()
    }

    /// Get all steps of type `S` from current state.
    pub fn steps_for<S: Step>(&self) -> impl DoubleEndedIterator<Item = (&S, StepStatus)> {
        self.0
            .iter()
            .filter_map(|(step, status)| Some((step.downcast_ref()?, *status)))
    }
}

#[async_trait(?Send)]
impl World for State {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
}

#[tokio::main]
async fn main() {
    let pool = connection_test_pool().await.unwrap();

    Cucumber::<State>::new()
        // Specifies where our feature files exist
        .features(&["./features/admin", "./features/addresses"])
        // Adds the implementation of our steps to the runner
        .steps(steps::download::steps())
        .steps(steps::admin::steps())
        .steps(steps::address::steps())
        .steps(steps::search::steps())
        // Add some global context for all the tests, like databases.
        .context(Context::new().add(pool))
        // Parses the command line arguments if passed
        .cli()
        // Runs the Cucumber tests and then exists
        .run_and_exit()
        .await
}
