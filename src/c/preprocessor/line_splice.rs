/*
   Handles splicing together of physical source lines to form logical source lines.

   This encompasses Translation Phase 2 of the C standard.

   Each line that ends in a backslash will be joined with the following line
*/

use crate::{
    ast::Source,
    text::{Character, Text, TextStream},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct LineSplicer<T: Text> {
    text: Rc<RefCell<T>>,
}

impl<T: Text> LineSplicer<T> {
    pub fn new(text: T) -> Self {
        Self {
            text: Rc::new(RefCell::new(text)),
        }
    }

    pub fn next_line(&self) -> Result<Line<T>, Source> {
        // We share a text source with the line we produce
        // in order to avoid needing to allocate memory for it.
        // So, we must ensure that no preivous lines still are being used.
        assert_eq!(Rc::strong_count(&self.text), 1);

        match RefCell::borrow_mut(&self.text).peek() {
            Character::At(_, _) => Ok(Line {
                text: LineSource::Text(self.text.clone()),
            }),
            Character::End(source) => Err(source),
        }
    }
}

#[derive(Debug)]
enum LineSource<T: Text> {
    Text(Rc<RefCell<T>>),
    End(Source),
}

#[derive(Debug)]
pub struct Line<T: Text> {
    text: LineSource<T>,
}

impl<T: Text> Line<T> {
    pub fn source(&self) -> Source {
        match &self.text {
            LineSource::Text(text) => RefCell::borrow_mut(text).source(),
            LineSource::End(source) => *source,
        }
    }
}

impl<T> Drop for Line<T>
where
    T: Text,
{
    fn drop(&mut self) {
        // We need to consume the rest of the line, since we share a text source and
        // the following line will need to start at its beginning.
        loop {
            match self.text {
                LineSource::Text(_) => {
                    let _ = self.next();
                }
                LineSource::End(_) => break,
            }
        }
    }
}

impl<T: Text> TextStream for Line<T> {
    fn next(&mut self) -> Character {
        match &self.text {
            LineSource::Text(text) => {
                let character = loop {
                    let mut text = RefCell::borrow_mut(text);
                    match text.next() {
                        Character::At('\n', source) | Character::End(source) => {
                            break Character::End(source);
                        }
                        Character::At('\\', _) if text.eat("\n") => (),
                        Character::At(c, source) => {
                            break Character::At(c, source);
                        }
                    }
                };

                if let Character::End(source) = character {
                    self.text = LineSource::End(source);
                }

                character
            }
            LineSource::End(source) => Character::End(*source),
        }
    }
}
