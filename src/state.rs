use crate::psmove::Controller;
use crate::games::Game;
use crate::sound::Sound;

pub struct Data {
    pub game: Game,
    pub sound: Sound,
    pub controllers: Vec<Controller>,
}

pub struct Event {}

pub trait State {
    fn on_start(&mut self, _: &mut Data) {}
    fn on_stop(&mut self, _: &mut Data) {}

    fn on_pause(&mut self, _: &mut Data) {}
    fn on_resume(&mut self, _: &mut Data) {}

    fn on_event(&mut self, _: &mut Data, _: Event) -> Transition { return Transition::None; }

    fn on_update(&mut self, _: &mut Data) -> Transition { return Transition::None; }
}

pub enum Transition {
    /// Continue without change.
    None,

    /// Remove the active state and resume the next state on the stack or shead if there are none.
    Pop,

    /// Pause the active state and push a new state onto the stack.
    Push(Box<dyn State>),

    /// Replace head-most state with the given one.
    Replace(Box<dyn State>),

    /// Remove all states on the stack and start with a new stack.
    ReplaceAll(Vec<Box<dyn State>>),

    /// Execute a series of Transitions.
    Sequence(Vec<Transition>),

    /// Remove all states and shut down the engine.
    Quit,
}

pub struct StateMachine {
    stack: Vec<Box<dyn State>>,
}

impl StateMachine {
    pub fn new<S>(mut initial_state: S,
                  data: &mut Data) -> Self
        where
            S: State + 'static,
    {
        initial_state.on_start(data);

        return Self {
            stack: vec![Box::new(initial_state)],
        };
    }

    pub fn is_running(&self) -> bool {
        return !self.stack.is_empty();
    }

    pub fn handle_event(&mut self, data: &mut Data, event: Event) {
        let trans = match self.stack.last_mut() {
            Some(state) => state.on_event(data, event),
            None => Transition::None,
        };

        self.transition(trans, data);
    }

    pub fn update(&mut self, data: &mut Data) {
        let trans = match self.stack.last_mut() {
            Some(state) => state.on_update(data),
            None => Transition::None,
        };

        self.transition(trans, data);
    }

    pub fn transition(&mut self, transition: Transition, data: &mut Data) {
        match transition {
            Transition::None => {}

            Transition::Pop => {
                if let Some(mut prev) = self.stack.pop() {
                    prev.on_stop(data);
                }
                if let Some(head) = self.stack.last_mut() {
                    head.on_resume(data);
                }
            }

            Transition::Push(mut next) => {
                if let Some(head) = self.stack.last_mut() {
                    head.on_pause(data);
                }
                next.on_start(data);
                self.stack.push(next);
            }

            Transition::Replace(mut next) => {
                if let Some(mut prev) = self.stack.pop() {
                    prev.on_stop(data);
                }
                next.on_start(data);
                self.stack.push(next);
            }

            Transition::ReplaceAll(stack) => {
                while let Some(mut prev) = self.stack.pop() {
                    prev.on_stop(data);
                }
                for mut next in stack {
                    if let Some(head) = self.stack.last_mut() {
                        head.on_pause(data);
                    }
                    next.on_start(data);
                    self.stack.push(next);
                }
            }

            Transition::Sequence(transitions) => {
                for transition in transitions {
                    self.transition(transition, data);
                }
            }

            Transition::Quit => {
                while let Some(mut prev) = self.stack.pop() {
                    prev.on_stop(data);
                }
            }
        }
    }
}