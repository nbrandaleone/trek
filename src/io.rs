//! Stackless coroutine I/O bridge (`IoHandle`) and output/paging primitives.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use crate::state::{GameState, SFAIR};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptType {
    Line,
    Key,
    Exited,
}

impl PromptType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PromptType::Line => "line",
            PromptType::Key => "key",
            PromptType::Exited => "exited",
        }
    }
}

#[derive(Clone, Debug)]
pub enum InputEvent {
    Line(String),
    Key(char),
}

#[derive(Debug, Default)]
pub(crate) struct IoShared {
    pub(crate) parked_state: Option<GameState>,
    pub(crate) waiting_for: Option<PromptType>,
    pub(crate) input: Option<InputEvent>,
}

#[derive(Clone, Debug)]
pub struct IoHandle {
    shared: Rc<RefCell<IoShared>>,
}

impl Default for IoHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl IoHandle {
    pub fn new() -> Self {
        Self {
            shared: Rc::new(RefCell::new(IoShared::default())),
        }
    }

    pub(crate) fn shared(&self) -> &Rc<RefCell<IoShared>> {
        &self.shared
    }

    pub(crate) fn park_finished(&self, mut state: GameState) {
        state.exited = true;
        state.in_game = false;
        let mut s = self.shared.borrow_mut();
        s.parked_state = Some(state);
        s.waiting_for = Some(PromptType::Exited);
    }

    /// Suspend until a line of text input is provided to the session.
    pub async fn read_line(&self, state: &mut GameState) -> String {
        {
            let mut s = self.shared.borrow_mut();
            let parked = std::mem::replace(state, GameState::empty_placeholder());
            s.parked_state = Some(parked);
            s.waiting_for = Some(PromptType::Line);
        }
        let line = ReadLineFuture {
            shared: self.shared.clone(),
        }
        .await;
        {
            let mut s = self.shared.borrow_mut();
            if let Some(parked) = s.parked_state.take() {
                *state = parked;
            }
        }
        line
    }

    /// Suspend until a single keypress is provided to the session.
    pub async fn getch(&self, state: &mut GameState) -> char {
        {
            let mut s = self.shared.borrow_mut();
            let parked = std::mem::replace(state, GameState::empty_placeholder());
            s.parked_state = Some(parked);
            s.waiting_for = Some(PromptType::Key);
        }
        let ch = GetChFuture {
            shared: self.shared.clone(),
        }
        .await;
        {
            let mut s = self.shared.borrow_mut();
            if let Some(parked) = s.parked_state.take() {
                *state = parked;
            }
        }
        ch
    }
}

struct ReadLineFuture {
    shared: Rc<RefCell<IoShared>>,
}

impl Future for ReadLineFuture {
    type Output = String;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut s = self.shared.borrow_mut();
        match s.input.take() {
            Some(InputEvent::Line(line)) => {
                s.waiting_for = None;
                Poll::Ready(line)
            }
            Some(InputEvent::Key(ch)) => {
                s.waiting_for = None;
                Poll::Ready(ch.to_string())
            }
            None => Poll::Pending,
        }
    }
}

struct GetChFuture {
    shared: Rc<RefCell<IoShared>>,
}

impl Future for GetChFuture {
    type Output = char;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut s = self.shared.borrow_mut();
        match s.input.take() {
            Some(InputEvent::Key(ch)) => {
                s.waiting_for = None;
                Poll::Ready(ch)
            }
            Some(InputEvent::Line(line)) => {
                s.waiting_for = None;
                Poll::Ready(line.chars().next().unwrap_or(' '))
            }
            None => Poll::Pending,
        }
    }
}

pub async fn pause(state: &mut GameState, io: &IoHandle, i: i32) {
    if state.a.iscloaked || state.skip_pauses {
        return;
    }
    state.proutn("\n");
    if i == 1 {
        if state.a.skill > SFAIR {
            state.proutn("[ANNOUNCEMENT ARRIVING...]\n");
        } else {
            state.proutn("[IMPORTANT ANNOUNCEMENT ARRIVING -- HIT SPACE BAR TO CONTINUE]\n");
        }
        io.getch(state).await;
    } else {
        if state.a.skill > SFAIR {
            state.proutn("[CONTINUE?]");
        } else {
            state.proutn("[HIT SPACE BAR TO CONTINUE]");
        }
        io.getch(state).await;
        state.proutn("\r                           \r");
    }
    if i != 0 {
        state.clearscreen();
    }
    state.linecount = 0;
}

pub async fn skip(state: &mut GameState, io: &IoHandle, mut i: i32) {
    while i > 0 {
        i -= 1;
        state.linecount += 1;
        if state.auto_page && state.linecount >= 23 {
            pause(state, io, 0).await;
        } else {
            state.proutn("\n");
        }
    }
}

pub async fn prout(state: &mut GameState, io: &IoHandle, s: &str) {
    state.proutn(s);
    skip(state, io, 1).await;
}

pub fn prouts(state: &mut GameState, s: &str) {
    state.proutn(s);
}

pub async fn stars(state: &mut GameState, io: &IoHandle) {
    prouts(state, "******************************************************");
    skip(state, io, 1).await;
}

pub async fn huh(state: &mut GameState, io: &IoHandle) {
    state.chew();
    skip(state, io, 1).await;
    prout(state, io, "Beg your pardon, Captain?").await;
}
