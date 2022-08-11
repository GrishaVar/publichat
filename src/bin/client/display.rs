use std::io::Write;
use std::time::{self, SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex, mpsc};
use std::mem;

use crossterm::{ExecutableCommand, QueueableCommand};
use crossterm::cursor;
use crossterm::style::{
    style,
    SetAttribute,
    Stylize,
    Attribute,
    PrintStyledContent,
    Color,
};
use crossterm::terminal::{
    self,
    ClearType,
    EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::event::{
    self,
    KeyEvent,
    MouseEvent,
};

use crate::common::*;

const BG_COLOUR: Color = Color::Rgb{r: 0xd0, g: 0xd0, b: 0xd0};
const FG_COLOUR: Color = Color::Rgb{r: 0x66, g: 0x00, b: 0x33};

enum ViewPos {
    Last,  // "most recent message on bottom"
    Index{msg_id: u16, chr_id: u8},  // id of TOP message, index of its first char
}

pub struct Display<'a> {
    state: Arc<Mutex<GlobalState>>,
    msg_tx: mpsc::Sender<String>,
    stdout: std::io::Stdout,
    size: (u16, u16),  // size of terminal (w, h)
    user_msg: String,  // stuff the user is typing
    view: ViewPos,
    last_update: SystemTime,
    chat_name: &'a str,
}

// WARNING: this file is very OO; proceed with your own risk!
impl<'a> Display<'a> {
    pub fn start(
        state: Arc<Mutex<GlobalState>>,
        msg_tx: mpsc::Sender<String>,
        chat_name: &str,
    ) -> crossterm::Result<()> {
        // setup
        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        stdout.queue(event::EnableMouseCapture)?;
        stdout.queue(EnterAlternateScreen)?;
        stdout.queue(terminal::Clear(ClearType::All))?;
        stdout.queue(cursor::DisableBlinking)?;
        stdout.queue(cursor::Hide)?;
        stdout.flush()?;

        // set up struct
        let mut disp = Display{
            state,
            msg_tx,
            stdout: std::io::stdout(),
            size: terminal::size()?,
            user_msg: String::with_capacity(50),
            view: ViewPos::Last,
            last_update: UNIX_EPOCH,
            chat_name,
        };

        // draw first frame
        disp.refresh()?;

        // enter mainloop
        disp.mainloop()?;

        // clean up
        stdout.queue(cursor::Show)?;
        stdout.queue(cursor::EnableBlinking)?;
        stdout.queue(LeaveAlternateScreen)?;
        stdout.queue(SetAttribute(Attribute::Reset))?;
        stdout.queue(event::DisableMouseCapture)?;
        stdout.flush()?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    fn mainloop(&mut self) -> crossterm::Result<()> {
        use crossterm::event::Event::*;
        use crossterm::event::KeyCode::{Char, Esc};
        use crossterm::event::KeyModifiers as Mod;
        loop {
            // wait for events to come in. Update instantly if they do, otherwise
            // update at defined FPS when a new message comes in.
            match event::poll(_DISP_DELAY) {
                Ok(true) => match event::read()? {
                    Key(KeyEvent{code: Char('c'), modifiers: Mod::CONTROL }) => break Ok(()),
                    Key(KeyEvent{code: Esc, modifiers: Mod::NONE }) => break Ok(()),
                    Key(e) => self.handle_keyboard_event(e)?,
                    Mouse(e) => self.handle_mouse_event(e)?,
                    Resize(x, y) => {
                        // I test this a lot but actually it should be a rare event
                        self.size = (x, y);
                        self.refresh()?;
                    },
                },
                Ok(false) => {},  // No events to be processed
                Err(e) => break Err(e),  // Failed to read, clean up and exit
            }
    
            // re-draw all messages from time to time
            // this will display new messages as they come in
            if time::SystemTime::now().duration_since(self.last_update).unwrap() > _DISP_DELAY {
                self.draw_messages()?;
                self.draw_footer()?;
                self.last_update = time::SystemTime::now();
            }
        }
    }

    fn draw_header(&mut self) -> crossterm::Result<()> {
        let (w, _h) = self.size;

        let mut stdout = std::io::stdout();
        stdout.queue(cursor::MoveTo(0, 0))?;

        let header_text = {
            let title = format!("> PubliChat: {} <", self.chat_name);
            let signs = "=".repeat((w as usize - title.len())/2);
            let extra = if ((w as usize - title.len()) & 1)==1 {"="} else {""};
            format!("{signs}{title}{signs}{extra}")
        };
        let header = style(header_text)
            .with(FG_COLOUR)
            .on(BG_COLOUR)
            .attribute(Attribute::Bold);

        stdout.queue(PrintStyledContent(header))?;

        let coloured_line = style(" ".repeat(w as usize))
            .on(FG_COLOUR);

        stdout.queue(cursor::MoveToNextLine(1))?;
        stdout.queue(PrintStyledContent(coloured_line))?;

        stdout.flush()
    }

    fn draw_footer(&mut self) -> crossterm::Result<()> {
        // TODO: account for max msg length! here or there?
        let mut stdout = std::io::stdout();
        let (w, h) = terminal::size()?;

        // draw purple separator
        stdout.queue(cursor::MoveTo(0, h-2))?;
        stdout.queue(PrintStyledContent(style(" ".repeat(w as usize)).on(FG_COLOUR)))?;

        // draw current input text
        stdout.queue(cursor::MoveToNextLine(1))?;
        stdout.queue(terminal::Clear(ClearType::CurrentLine))?;  // del line only
        let blinker = style("> ")
            .bold()
            .rapid_blink()
            .with(FG_COLOUR)
            .on(BG_COLOUR);
        stdout.queue(PrintStyledContent(blinker))?;
        let text = style(&self.user_msg)
            .with(FG_COLOUR)
            .on(BG_COLOUR);
        stdout.queue(PrintStyledContent(text))?;
        let spaces = style(" ".repeat(w as usize - 2 - self.user_msg.len()))
            .on(BG_COLOUR);
        stdout.queue(PrintStyledContent(spaces))?;
        stdout.flush()
    }

    fn draw_messages(&mut self) -> crossterm::Result<()> {
        // SIDE EFFECT: DELETES FOOTER!!!
        let state = self.state.lock().unwrap();  // TODO get rid of this unwrap!!!
        if state.queue.is_empty() {return Ok(())}

        let mut stdout = std::io::stdout();

        let (w, h) = self.size;
        let mut remaining_lines = h - 4;  // two lines used on top, two on bottom

        // TODO: find a way of changing backgroud nicely
        // stdout.queue(SetForegroundColor(Color::Black))?;
        // stdout.queue(SetBackgroundColor(Color::Grey))?;

        // clear current screen  (THIS DELETES FOOTER!)
        stdout.queue(cursor::MoveTo(0, 2))?;  // TODO: terminal too small?
        stdout.queue(terminal::Clear(ClearType::FromCursorDown))?;

        if state.queue.len() <= remaining_lines as usize &&
            // it's possible all messages fit on the screen
            // do more expensive check to see if it's true
            state.queue.iter().map(|m| 1+(m.repr.len() as u16 / w)).sum::<u16>() < h
        {
            // if it is true, just print with no checks
            for msg in state.queue.iter() {
                write!(stdout, "{msg}\r\n")?;
            }
            return stdout.flush();
        }
        
        // Not all messages fit on screen
        match self.view {
            ViewPos::Index { msg_id, .. } => {  // TODO: use chr_id
                // cursor already at right position, draw one msg at a time
                // TODO: print start.msg partial
                for msg in state.queue.range(1+usize::from(msg_id)..) {
                    if remaining_lines == 0 { break }

                    let line_count = (msg.repr.len() as u16 / w) + 1;
                    if let Some(res) = remaining_lines.checked_sub(line_count) {
                        // normal situation, whole message fits on screen
                        write!(stdout, "{msg}\r\n")?;
                        remaining_lines = res;
                    } else {
                        let printable_chars = remaining_lines * w;
                        write!(stdout, "{}", &msg.to_string()[..usize::from(printable_chars)])?;
                        break;  // finished drawing
                    }
                }
            }
            ViewPos::Last => {
                // draw from bottom up
                stdout.queue(cursor::MoveTo(0, h-2))?;
                for msg in state.queue.iter().rev() {
                    if remaining_lines == 0 { break }
                    let msg_height = (msg.repr.len() as u16 / w) + 1;
                    if msg_height <= remaining_lines {  // message fits no problemo
                        stdout.queue(cursor::MoveToPreviousLine(msg_height))?;
                        write!(stdout, "{msg}")?;
                        remaining_lines -= msg_height;
                    } else {  // only bottom half of top msg fits
                        // stdout.queue(cursor::MoveToPreviousLine(remaining_lines))?;
                        
                        stdout.queue(cursor::MoveTo(0, 2))?;
                        let skipped_lines = msg_height - remaining_lines;
                        write!(stdout, "{}", &msg.repr[(w*skipped_lines) as usize..])?;
                        break;  // finished drawing
                    }
                }
            }
        }
        stdout.flush()
    }

    fn refresh(&mut self) -> crossterm::Result<()> {
        // clear & draw the full frame
        self.stdout.execute(terminal::Clear(ClearType::All))?;
        self.draw_header()?;
        self.draw_messages()?;
        self.draw_footer()
    }

    fn move_pos(&mut self, up: bool) {
        // positive is scolling up
        self.view = match self.view {
            ViewPos::Last => ViewPos::Last,
            ViewPos::Index{msg_id, chr_id} => if up {
                ViewPos::Index{msg_id: msg_id-1, chr_id}
            } else {
                // TODO: possible ViewPos::Last
                ViewPos::Index{msg_id: msg_id+1, chr_id}
            },
        };
    }

    fn handle_keyboard_event(&mut self, event: KeyEvent) -> crossterm::Result<()> {
        use crossterm::event::KeyCode::*;
        use crossterm::event::KeyModifiers as Mod;
        match (event.modifiers, event.code) {
            (Mod::NONE, Char(c)) | (Mod::SHIFT, Char(c)) => {  // add char
                self.user_msg.push(c);
                self.draw_footer()
            },
            (Mod::NONE, Backspace) => {  // remove char
                self.user_msg.pop();
                self.draw_footer()
            },
            (Mod::NONE, Enter) => {  // send message
                use std::io::{Error, ErrorKind::Other};
                self.msg_tx.send(mem::take(&mut self.user_msg))
                    .map_err(|_| Error::new(Other, "msg_rx closed"))?;
                self.draw_footer()
            },
            // (Mod::CONTROL, Backspace) => Ok(()),  // remove word
            // (Mod::NONE, Delete) => Ok(()),  // remove char
            // (Mod::CONTROL, Delete) => Ok(()),  // remove word
            (Mod::NONE, Up) => {  // scroll up
                self.move_pos(true);
                self.draw_messages()?;
                self.draw_footer()
            },
            (Mod::NONE, Down) => {  // scroll down
                self.move_pos(false);
                self.draw_messages()?;
                self.draw_footer()
            },
            (Mod::NONE, PageUp) => Ok(()),  // scroll way up
            (Mod::NONE, PageDown) => Ok(()),  // scroll way down
            (Mod::NONE, Home) => {  // scroll way way up
                self.view = ViewPos::Index{msg_id: 0, chr_id: 0};
                self.draw_messages()?;
                self.draw_footer()
            },
            (Mod::NONE, End) => {  // scroll way way down
                self.view = ViewPos::Last;
                self.draw_messages()?;
                self.draw_footer()
            },
            (Mod::CONTROL, Char('r')) => self.refresh(),  // redraw everything
            (Mod::CONTROL, Char('c')) | (Mod::NONE, Esc) => unreachable!(),
            _ => Ok(()),
        }
    }

    fn handle_mouse_event(&mut self, event: MouseEvent) -> crossterm::Result<()> {
        use crossterm::event::MouseEventKind::*;
        match event.kind {
            ScrollUp => {
                self.move_pos(true);
                self.draw_messages()?;
                self.draw_footer()
            },
            ScrollDown => {
                self.move_pos(false);
                self.draw_messages()?;
                self.draw_footer()
            },
            _ => Ok(()),
        }
    }
}
