//! The engine is the core piece of every good fuzzer

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt::Debug;
use core::marker::PhantomData;

use hashbrown::HashMap;

use crate::corpus::{Corpus, HasCorpus, Testcase};
use crate::events::EventManager;
use crate::executors::Executor;
use crate::feedbacks::Feedback;
use crate::inputs::Input;
use crate::observers::Observer;
use crate::stages::Stage;
use crate::utils::Rand;
use crate::AflError;

// TODO FeedbackMetadata to store histroy_map

pub trait StateMetadata: Debug {
    /// The name of this metadata - used to find it in the list of avaliable metadatas
    fn name(&self) -> &'static str;
}

pub trait State<C, E, I, R>: HasCorpus<C, I, R>
where
    C: Corpus<I, R>,
    E: Executor<I>,
    I: Input,
    R: Rand,
{
    /// Get executions
    fn executions(&self) -> usize;

    /// Set executions
    fn set_executions(&mut self, executions: usize);

    /// Get all the metadatas into an HashMap
    fn metadatas(&self) -> &HashMap<&'static str, Box<dyn StateMetadata>>;

    /// Get all the metadatas into an HashMap (mutable)
    fn metadatas_mut(&mut self) -> &mut HashMap<&'static str, Box<dyn StateMetadata>>;

    /// Add a metadata
    fn add_metadata(&mut self, meta: Box<dyn StateMetadata>) {
        self.metadatas_mut().insert(meta.name(), meta);
    }

    /// Get the linked observers
    fn observers(&self) -> &[Rc<RefCell<dyn Observer>>];

    /// Get the linked observers
    fn observers_mut(&mut self) -> &mut Vec<Rc<RefCell<dyn Observer>>>;

    /// Add a linked observer
    fn add_observer(&mut self, observer: Rc<RefCell<dyn Observer>>) {
        self.observers_mut().push(observer);
    }

    /// Reset the state of all the observes linked to this executor
    fn reset_observers(&mut self) -> Result<(), AflError> {
        for observer in self.observers() {
            observer.borrow_mut().reset()?;
        }
        Ok(())
    }

    /// Run the post exec hook for all the observes linked to this executor
    fn post_exec_observers(&mut self) -> Result<(), AflError> {
        self.observers()
            .iter()
            .map(|x| x.borrow_mut().post_exec())
            .fold(Ok(()), |acc, x| if x.is_err() { x } else { acc })
    }

    /// Returns vector of feebacks
    fn feedbacks(&self) -> &[Box<dyn Feedback<I>>];

    /// Returns vector of feebacks (mutable)
    fn feedbacks_mut(&mut self) -> &mut Vec<Box<dyn Feedback<I>>>;

    /// Adds a feedback
    fn add_feedback(&mut self, feedback: Box<dyn Feedback<I>>) {
        self.feedbacks_mut().push(feedback);
    }

    /// Return the executor
    fn executor(&self) -> &E;

    /// Return the executor (mutable)
    fn executor_mut(&mut self) -> &mut E;

    /// Runs the input and triggers observers and feedback
    fn evaluate_input(&mut self, input: &I) -> Result<bool, AflError> {
        self.reset_observers()?;
        self.executor_mut().run_target(input)?;
        self.set_executions(self.executions() + 1);
        self.post_exec_observers()?;

        let mut rate_acc = 0;
        for feedback in self.feedbacks_mut() {
            rate_acc += feedback.is_interesting(input)?;
        }

        if rate_acc >= 25 {
            let testcase = Rc::new(RefCell::new(Testcase::new(input.clone())));
            for feedback in self.feedbacks_mut() {
                feedback.append_metadata(testcase.clone())?;
            }
            Ok(true)
        } else {
            for feedback in self.feedbacks_mut() {
                feedback.discard_metadata()?;
            }
            Ok(false)
        }
    }
}

pub struct StdState<C, E, I, R>
where
    C: Corpus<I, R>,
    E: Executor<I>,
    I: Input,
    R: Rand,
{
    executions: usize,
    metadatas: HashMap<&'static str, Box<dyn StateMetadata>>,
    // additional_corpuses: HashMap<&'static str, Box<dyn Corpus>>,
    observers: Vec<Rc<RefCell<dyn Observer>>>,
    feedbacks: Vec<Box<dyn Feedback<I>>>,
    corpus: C,
    executor: E,
    phantom: PhantomData<R>,
}

impl<C, E, I, R> HasCorpus<C, I, R> for StdState<C, E, I, R>
where
    C: Corpus<I, R>,
    E: Executor<I>,
    I: Input,
    R: Rand,
{
    fn corpus(&self) -> &C {
        &self.corpus
    }

    /// Get thecorpus field (mutable)
    fn corpus_mut(&mut self) -> &mut C {
        &mut self.corpus
    }
}

impl<C, E, I, R> State<C, E, I, R> for StdState<C, E, I, R>
where
    C: Corpus<I, R>,
    E: Executor<I>,
    I: Input,
    R: Rand,
{
    fn executions(&self) -> usize {
        self.executions
    }

    fn set_executions(&mut self, executions: usize) {
        self.executions = executions
    }

    fn metadatas(&self) -> &HashMap<&'static str, Box<dyn StateMetadata>> {
        &self.metadatas
    }

    fn metadatas_mut(&mut self) -> &mut HashMap<&'static str, Box<dyn StateMetadata>> {
        &mut self.metadatas
    }

    fn observers(&self) -> &[Rc<RefCell<dyn Observer>>] {
        &self.observers
    }

    fn observers_mut(&mut self) -> &mut Vec<Rc<RefCell<dyn Observer>>> {
        &mut self.observers
    }

    fn feedbacks(&self) -> &[Box<dyn Feedback<I>>] {
        &self.feedbacks
    }

    fn feedbacks_mut(&mut self) -> &mut Vec<Box<dyn Feedback<I>>> {
        &mut self.feedbacks
    }

    fn executor(&self) -> &E {
        &self.executor
    }

    fn executor_mut(&mut self) -> &mut E {
        &mut self.executor
    }
}

impl<C, E, I, R> StdState<C, E, I, R>
where
    C: Corpus<I, R>,
    E: Executor<I>,
    I: Input,
    R: Rand,
{
    pub fn new(corpus: C, executor: E) -> Self {
        StdState {
            executions: 0,
            metadatas: HashMap::default(),
            observers: vec![],
            feedbacks: vec![],
            corpus: corpus,
            executor: executor,
            phantom: PhantomData,
        }
    }
}

pub trait Engine<S, EM, E, C, I, R>
where
    S: State<C, E, I, R>,
    EM: EventManager,
    E: Executor<I>,
    C: Corpus<I, R>,
    I: Input,
    R: Rand,
{
    fn stages(&self) -> &[Box<dyn Stage<S, EM, E, C, I, R>>];

    fn stages_mut(&mut self) -> &mut Vec<Box<dyn Stage<S, EM, E, C, I, R>>>;

    fn add_stage(&mut self, stage: Box<dyn Stage<S, EM, E, C, I, R>>) {
        self.stages_mut().push(stage);
    }

    fn fuzz_one(
        &mut self,
        rand: &mut R,
        state: &mut S,
        events: &mut EM,
    ) -> Result<usize, AflError> {
        let (testcase, idx) = state.corpus_mut().next(rand)?;
        println!("Cur entry: {}\tExecutions: {}", idx, state.executions());
        for stage in self.stages_mut() {
            stage.perform(rand, state, events, testcase.clone())?;
        }
        Ok(idx)
    }
}

pub struct StdEngine<S, EM, E, C, I, R>
where
    S: State<C, E, I, R>,
    EM: EventManager,
    E: Executor<I>,
    C: Corpus<I, R>,
    I: Input,
    R: Rand,
{
    stages: Vec<Box<dyn Stage<S, EM, E, C, I, R>>>,
    phantom: PhantomData<EM>,
}

impl<S, EM, E, C, I, R> Engine<S, EM, E, C, I, R> for StdEngine<S, EM, E, C, I, R>
where
    S: State<C, E, I, R>,
    EM: EventManager,
    E: Executor<I>,
    C: Corpus<I, R>,
    I: Input,
    R: Rand,
{
    fn stages(&self) -> &[Box<dyn Stage<S, EM, E, C, I, R>>] {
        &self.stages
    }

    fn stages_mut(&mut self) -> &mut Vec<Box<dyn Stage<S, EM, E, C, I, R>>> {
        &mut self.stages
    }
}

impl<S, EM, E, C, I, R> StdEngine<S, EM, E, C, I, R>
where
    S: State<C, E, I, R>,
    EM: EventManager,
    E: Executor<I>,
    C: Corpus<I, R>,
    I: Input,
    R: Rand,
{
    pub fn new() -> Self {
        StdEngine {
            stages: vec![],
            phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {

    use alloc::boxed::Box;

    use crate::corpus::{Corpus, InMemoryCorpus, Testcase};
    use crate::engines::{Engine, StdEngine, StdState};
    use crate::events::LoggerEventManager;
    use crate::executors::inmemory::InMemoryExecutor;
    use crate::executors::{Executor, ExitKind};
    use crate::inputs::bytes::BytesInput;
    use crate::mutators::scheduled::{mutation_bitflip, ComposedByMutations, StdScheduledMutator};
    use crate::stages::mutational::StdMutationalStage;
    use crate::utils::StdRand;

    fn harness<I>(_executor: &dyn Executor<I>, _buf: &[u8]) -> ExitKind {
        ExitKind::Ok
    }

    #[test]
    fn test_engine() {
        let mut rand = StdRand::new(0);

        let mut corpus = InMemoryCorpus::<BytesInput, StdRand>::new();
        let testcase = Testcase::new(vec![0; 4]).into();
        corpus.add(testcase);

        let executor = InMemoryExecutor::<BytesInput>::new(harness);
        let mut state = StdState::new(corpus, executor);

        let mut events_manager = LoggerEventManager::new();

        let mut engine = StdEngine::new();
        let mut mutator = StdScheduledMutator::new();
        mutator.add_mutation(mutation_bitflip);
        let stage = StdMutationalStage::new(mutator);
        engine.add_stage(Box::new(stage));

        //

        for i in 0..1000 {
            engine
                .fuzz_one(&mut rand, &mut state, &mut events_manager)
                .expect(&format!("Error in iter {}", i));
        }
    }
}
