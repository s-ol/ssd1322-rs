use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

use display_interface::{DataFormat, DisplayError, WriteOnlyDataCommand};

#[macro_export]
macro_rules! send {
        ([$($d:tt),*]) => {$crate::testing::Sent::Data(std::vec![$($d,)*])};
        ($c:tt) => {$crate::testing::Sent::Cmd($c)};
    }

#[macro_export]
macro_rules! sends {
        ($($e:tt),*) => {&[$($crate::send!($e),)*]};
    }

#[derive(Clone, Debug, PartialEq)]
pub enum Sent {
    Cmd(u8),
    Data(Vec<u8>),
}

pub struct TestSpyInterface {
    sent: Rc<RefCell<Vec<Sent>>>,
}

impl TestSpyInterface {
    pub fn new() -> Self {
        TestSpyInterface {
            sent: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn split(&self) -> Self {
        Self {
            sent: self.sent.clone(),
        }
    }

    pub fn check(&self, cmd: u8, data: &[u8]) {
        let sent = self.sent.borrow();
        if data.len() == 0 {
            assert_eq!(sent.len(), 1);
        } else {
            assert_eq!(sent.len(), 2);
            assert_eq!(sent[1], Sent::Data(data.to_vec()));
        }
        assert_eq!(sent[0], Sent::Cmd(cmd));
    }

    pub fn check_multi(&self, expect: &[Sent]) {
        assert_eq!(*self.sent.borrow(), expect);
    }

    pub fn clear(&mut self) {
        self.sent.borrow_mut().clear()
    }
}

impl WriteOnlyDataCommand for TestSpyInterface {
    fn send_commands(&mut self, cmd: DataFormat<'_>) -> Result<(), DisplayError> {
        let mut commands = match cmd {
            DataFormat::U8(data) => data.iter().map(|c| Sent::Cmd(*c)).collect(),
            DataFormat::U8Iter(data) => data.map(Sent::Cmd).collect(),
            _ => todo!(),
        };
        self.sent.borrow_mut().append(&mut commands);
        Ok(())
    }

    fn send_data(&mut self, buf: DataFormat<'_>) -> Result<(), DisplayError> {
        self.sent.borrow_mut().push(match buf {
            DataFormat::U8(data) => Sent::Data(data.to_vec()),
            DataFormat::U8Iter(data) => Sent::Data(data.collect()),
            _ => todo!(),
        });
        Ok(())
    }
}
