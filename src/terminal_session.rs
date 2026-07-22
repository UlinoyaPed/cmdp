use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io;

pub(crate) trait TerminalOps {
    fn raw(&mut self, enabled: bool) -> Result<()>;
    fn alternate(&mut self, enabled: bool) -> Result<()>;
    fn mouse(&mut self, enabled: bool) -> Result<()>;
    fn cursor(&mut self, visible: bool) -> Result<()>;
}

pub(crate) struct SystemOps;

impl TerminalOps for SystemOps {
    fn raw(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            enable_raw_mode()?
        } else {
            disable_raw_mode()?
        }
        Ok(())
    }
    fn alternate(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            execute!(io::stdout(), EnterAlternateScreen)?;
        } else {
            execute!(io::stdout(), LeaveAlternateScreen)?;
        }
        Ok(())
    }
    fn mouse(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            execute!(io::stdout(), EnableMouseCapture)?;
        } else {
            execute!(io::stdout(), DisableMouseCapture)?;
        }
        Ok(())
    }
    fn cursor(&mut self, visible: bool) -> Result<()> {
        if visible {
            execute!(io::stdout(), Show)?;
        } else {
            execute!(io::stdout(), Hide)?;
        }
        Ok(())
    }
}

pub struct TerminalSession<T: TerminalOps = SystemOps> {
    ops: T,
    raw: bool,
    alternate: bool,
    mouse: bool,
    cursor_hidden: bool,
}

impl TerminalSession<SystemOps> {
    pub fn enter() -> Result<Self> {
        Self::enter_with(SystemOps)
    }
}

impl<T: TerminalOps> TerminalSession<T> {
    fn enter_with(ops: T) -> Result<Self> {
        let mut session = Self {
            ops,
            raw: false,
            alternate: false,
            mouse: false,
            cursor_hidden: false,
        };
        session.ops.raw(true)?;
        session.raw = true;
        session.ops.alternate(true)?;
        session.alternate = true;
        session.ops.mouse(true)?;
        session.mouse = true;
        session.ops.cursor(false)?;
        session.cursor_hidden = true;
        Ok(session)
    }

    pub fn restore(&mut self) -> Result<()> {
        let mut first = None;
        macro_rules! attempt {
            ($expr:expr) => {
                if let Err(error) = $expr {
                    if first.is_none() {
                        first = Some(error);
                    }
                }
            };
        }
        if self.raw {
            attempt!(self.ops.raw(false));
            self.raw = false;
        }
        if self.mouse {
            attempt!(self.ops.mouse(false));
            self.mouse = false;
        }
        if self.alternate {
            attempt!(self.ops.alternate(false));
            self.alternate = false;
        }
        if self.cursor_hidden {
            attempt!(self.ops.cursor(true));
            self.cursor_hidden = false;
        }
        first.map_or(Ok(()), Err)
    }
}

impl<T: TerminalOps> Drop for TerminalSession<T> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    #[derive(Default)]
    struct Fake {
        calls: Rc<RefCell<Vec<String>>>,
        fail_alt: bool,
    }
    impl TerminalOps for Fake {
        fn raw(&mut self, on: bool) -> Result<()> {
            self.calls.borrow_mut().push(format!("raw:{on}"));
            Ok(())
        }
        fn alternate(&mut self, on: bool) -> Result<()> {
            self.calls.borrow_mut().push(format!("alt:{on}"));
            if on && self.fail_alt {
                anyhow::bail!("alt")
            };
            Ok(())
        }
        fn mouse(&mut self, on: bool) -> Result<()> {
            self.calls.borrow_mut().push(format!("mouse:{on}"));
            Ok(())
        }
        fn cursor(&mut self, on: bool) -> Result<()> {
            self.calls.borrow_mut().push(format!("cursor:{on}"));
            Ok(())
        }
    }
    #[test]
    fn partial_initialization_rolls_back() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let ops = Fake {
            calls: calls.clone(),
            fail_alt: true,
        };
        assert!(TerminalSession::enter_with(ops).is_err());
        let calls = calls.borrow();
        assert!(calls.contains(&"raw:true".into()));
        assert!(calls.contains(&"alt:true".into()));
        assert!(calls.contains(&"raw:false".into()));
        assert!(!calls.contains(&"mouse:false".into()));
    }
}
