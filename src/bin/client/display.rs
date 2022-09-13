use std::io::{self, Write};
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
};
use crossterm::event;

use crate::common::*;

const BG_COLOUR: Color = Color::Rgb{r: 0xd0, g: 0xd0, b: 0xd0};
const FG_COLOUR: Color = Color::Rgb{r: 0x66, g: 0x00, b: 0x33};

const HEADER_HEIGHT: u16 = 2;
const FOOTER_HEIGHT: u16 = 2;
const MIN_HEIGHT: u16 = HEADER_HEIGHT + 1 + FOOTER_HEIGHT;
const MIN_WIDTH: u16 = 25;

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
        stdout.queue(terminal::EnterAlternateScreen)?;
        stdout.queue(terminal::Clear(ClearType::All))?;
        stdout.queue(cursor::DisableBlinking)?;
        stdout.queue(cursor::Hide)?;
        stdout.queue(terminal::SetTitle("PubliChat"))?;
        stdout.flush()?;

        // set up struct
        let mut disp = Display{
            state,
            msg_tx,
            stdout: std::io::stdout(),
            size: terminal::size()?,
            user_msg: String::with_capacity(50),
            view: ViewPos::Index { msg_id: 0, chr_id: 0 },
            last_update: UNIX_EPOCH,
            chat_name,
        };

        // draw first frame then start mainloop
        // preserve errors for returning, but don't return yet
        // (cleanup will still be needed)
        let res = disp.refresh().and(disp.mainloop());

        // clean up
        stdout.queue(cursor::Show)?;
        stdout.queue(cursor::EnableBlinking)?;
        stdout.queue(terminal::LeaveAlternateScreen)?;
        stdout.queue(SetAttribute(Attribute::Reset))?;
        stdout.queue(event::DisableMouseCapture)?;
        stdout.flush()?;
        terminal::disable_raw_mode()?;

        res
    }

    fn mainloop(&mut self) -> crossterm::Result<()> {
        use crossterm::event::Event::*;
        use crossterm::event::KeyEvent as KE;
        use crossterm::event::KeyCode::{Char, Esc};
        use crossterm::event::KeyModifiers as Mod;
        loop {
            // wait for events to come in. Update instantly if they do, otherwise
            // update at defined FPS when a new message comes in.
            match event::poll(_DISP_DELAY) {
                Ok(true) => match event::read()? {
                    Key(KE{code: Char('c'), modifiers: Mod::CONTROL, .. }) => break Ok(()),
                    Key(KE{code: Char('d'), modifiers: Mod::CONTROL, .. }) => break Ok(()),
                    Key(KE{code: Char('z'), modifiers: Mod::CONTROL, .. }) => break Ok(()),
                    Key(KE{code: Esc,       modifiers: Mod::NONE,    .. }) => break Ok(()),
                    Key(e) => self.handle_keyboard_event(e)?,
                    Mouse(e) => self.handle_mouse_event(e)?,
                    Resize(x, y) => self.handle_resize(x, y)?,
                    Paste(s) => self.handle_paste(s)?,
                    FocusGained => {},  // TODO
                    FocusLost => {},  // TODO
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
        let w = self.size.0 as usize;

        let mut stdout = std::io::stdout();
        stdout.queue(cursor::MoveTo(0, 0))?;

        // TODO: cache with each size change?
        // TODO: make hidable?
        let header_text = format!("{:^w$}", self.chat_name);
        let header = style(header_text)
            .with(FG_COLOUR)
            .on(BG_COLOUR)
            .attribute(Attribute::Bold);

        stdout.queue(PrintStyledContent(header))?;

        let coloured_line = style(" ".repeat(w)).on(FG_COLOUR);

        stdout.queue(cursor::MoveToNextLine(1))?;
        stdout.queue(PrintStyledContent(coloured_line))?;

        stdout.flush()
    }

    fn draw_footer(&mut self) -> crossterm::Result<()> {
        // TODO: notify when message too long
        let mut stdout = std::io::stdout();
        let (w, h) = terminal::size()?;
        let max_text_len = w as usize - 2;

        // draw purple separator
        stdout.queue(cursor::MoveTo(0, h - 2))?;
        stdout.queue(PrintStyledContent(
            style(" ".repeat(w as usize)).on(FG_COLOUR)
        ))?;

        // draw current input text
        stdout.queue(cursor::MoveToNextLine(1))?;
        stdout.queue(terminal::Clear(ClearType::CurrentLine))?;  // del line only

        // print blinker
        stdout.queue(PrintStyledContent(
            style("> ").bold().rapid_blink().with(FG_COLOUR).on(BG_COLOUR)
        ))?;

        // print user's typed message
        let vis_text = match self.user_msg.char_indices().rev().nth(max_text_len - 1) {
            // slice of last max_text_len charachters of typed message
            None => self.user_msg.as_str(),  // shorter than max => show whole
            Some((i, _)) => &self.user_msg[i..],
        };
        stdout.queue(PrintStyledContent(
            style(vis_text).with(FG_COLOUR).on(BG_COLOUR)
        ))?;
        if vis_text.len() == self.user_msg.len() {
            // vis and text have same length => text wasn't shortened
            let spaces_len = max_text_len - self.user_msg.chars().count();
            stdout.queue(PrintStyledContent(
                style(" ".repeat(spaces_len)).on(BG_COLOUR)
            ))?;
        }
        stdout.flush()
    }

    fn draw_messages(&mut self) -> crossterm::Result<()> {
        // SIDE EFFECT: DELETES FOOTER!!!
        let state = self.state.lock().map_err(|_| {
            use std::io::{Error, ErrorKind::Other};
            Error::new(Other, "Failed to lock state")
        })?;
        if state.queue.is_empty() {return Ok(())}

        let mut stdout = std::io::stdout();

        let (w, h) = self.size;
        let mut remaining_lines = h - (HEADER_HEIGHT + FOOTER_HEIGHT);

        // TODO: find a way of changing backgroud nicely
        // stdout.queue(SetForegroundColor(Color::Black))?;
        // stdout.queue(SetBackgroundColor(Color::Grey))?;

        // TODO: this can just be u16::div_ceil once that stabilises. rust#88581
        let req_lines = |len| if len % w > 0 { len / w + 1 } else { len / w };

        // clear current screen  (THIS DELETES FOOTER!)
        stdout.queue(cursor::MoveTo(0, HEADER_HEIGHT))?;  // TODO: terminal too small?
        stdout.queue(terminal::Clear(ClearType::FromCursorDown))?;

        if state.queue.len() <= remaining_lines as usize &&
            // it's possible all messages fit on the screen
            // do more expensive check to see if it's true
            state.queue.iter().map(|m| req_lines(m.len)).sum::<u16>() <= remaining_lines
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
                if msg_id >= state.queue.len() as u16 { return Ok(()) }  // too far down
                for msg in state.queue.range(1 + usize::from(msg_id)..) {
                    let msg_height = req_lines(msg.len);
                    if msg_height <= remaining_lines {
                        // normal situation, whole message fits on screen
                        write!(stdout, "{msg}\r\n")?;
                        remaining_lines -= msg_height;  // TODO: check cursor pos?
                    } else {
                        // TODO: think about dealing with unicode graphemes
                        // let printable_chars = remaining_lines * w;
                        // write!(stdout, "{}", &msg.to_string()[..usize::from(printable_chars)])?;
                        break;  // finished drawing
                    }
                }
            }
            ViewPos::Last => {
                // draw from bottom up
                // last message shown in full, but top message might be cut off
                stdout.queue(cursor::MoveTo(0, h - FOOTER_HEIGHT))?;
                for msg in state.queue.iter().rev() {
                    let msg_height = req_lines(msg.len);
                    if msg_height <= remaining_lines {
                        // message fits no problemo. Move up to fit it:
                        stdout.queue(cursor::MoveToPreviousLine(msg_height))?;

                        // print, then return to starting point:
                        stdout.queue(cursor::SavePosition)?;
                        write!(stdout, "{msg}")?;
                        stdout.queue(cursor::RestorePosition)?;

                        remaining_lines -= msg_height;
                        if remaining_lines == 0 { break }
                    } else {
                        // only bottom half of top msg fits
                        // note: the message prefix will NOT be visible
                        stdout.queue(cursor::MoveTo(0, HEADER_HEIGHT))?;
                        let skipped_lines = msg_height - remaining_lines;

                        // TODO: think about dealing with unicode graphemes
                        // write!(stdout, "{}", &msg.repr[(w*skipped_lines) as usize..])?;
                        break;  // finished drawing
                    }
                }
            }
        }
        stdout.flush()
    }

    fn refresh(&mut self) -> crossterm::Result<()> {
        // Draw the full frame
        self.draw_header()?;
        self.draw_messages()?;
        self.draw_footer()?;
        self.stdout.flush()
    }

    fn move_pos(&mut self, up: bool) {
        // positive is scolling up
        self.view = match self.view {
            ViewPos::Last => ViewPos::Last,
            ViewPos::Index{mut msg_id, chr_id} => if up {
                msg_id = msg_id.checked_sub(1).unwrap_or(0);
                ViewPos::Index{ msg_id, chr_id }
            } else {
                // TODO: possible ViewPos::Last
                ViewPos::Index{ msg_id: msg_id+1, chr_id }
            },
        };
    }

    fn handle_keyboard_event(&mut self, event: event::KeyEvent) -> crossterm::Result<()> {
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

    fn handle_mouse_event(&mut self, event: event::MouseEvent) -> crossterm::Result<()> {
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

    fn handle_resize(&mut self, x: u16, y: u16) -> crossterm::Result<()> {
        if y < MIN_HEIGHT || x < MIN_WIDTH {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Terminal size not supported! Too small :(",
            ))
        }
        self.size = (x, y);
        self.refresh()
    }

    fn handle_paste(&mut self, s: String) -> crossterm::Result<()> {
        self.user_msg.push_str(&s);
        self.draw_footer()
    }
}
