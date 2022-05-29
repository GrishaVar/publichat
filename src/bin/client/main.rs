use std::collections::VecDeque;
use std::error::Error;
use std::io::Write;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::mem;
use std::time;

use crossterm::ExecutableCommand;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyEvent;
use crossterm::style::Stylize;
use crossterm::terminal::Clear;
use publichat::helpers::*;
use publichat::constants::*;

mod msg;
use msg::Message;

mod crypt;
mod comm;

const FQ_DELAY: Duration = Duration::from_millis(200);
const DISP_FPS: u64 = 1;
const _DISP_DELAY: Duration = Duration::from_millis(1000 / DISP_FPS);

fn parse_header(header: &[u8; HED_OUT_SIZE]) -> Result<(u8, u32, u8, bool), &'static str> {
    // returns (chat id byte, message id, message count, forward)
    if header[..PADDING_SIZE] == MSG_PADDING {
        Ok((
            header[HED_OUT_CHAT_ID_BYTE],  // TODO: poorly named consts here...
            u32::from_be_bytes(header[HED_OUT_CHAT_ID_BYTE..][..QUERY_ARG_SIZE].try_into().unwrap()) & 0x00_ff_ff_ff,
            header[HED_OUT_MSG_COUNT] & 0b0111_1111,  // can't fail unless consts wrong ^
            header[HED_OUT_MSG_COUNT] & 0b1000_0000 > 0,
        ))
    } else {
        println!("{header:?}");
        Err("Received invalid header padding")
    }
}

struct GlobalState {
    queue: VecDeque<Message>,
    chat_key: Hash,
    chat_id: Hash,
    user_id: Hash,
    min_id: u32,
    max_id: u32,  // inclusive
}

fn listener(mut stream: TcpStream, state: Arc<Mutex<GlobalState>>) -> Res {
    let mut hed_buf = [0; HED_OUT_SIZE];
    loop {
        read_exact(&mut stream, &mut hed_buf, "Failed to read head buffer")?;
        // TODO: what should happen when this fails?
        // I guess thread closes and require reconnect
        
        let (chat, first_id, count, forward) = parse_header(&hed_buf)?;
        if count == 0 { continue }  // skip no messages

        // read messages expected from header
        let mut buf = vec![0; count as usize * MSG_OUT_SIZE];  // TODO: consider array
        read_exact(&mut stream, &mut buf, "Failed to bulk read fetch")?;

        let mut s = state.lock().map_err(|_| "Failed to lock state")?;
        if chat != s.chat_id[0] { continue }  // skip wrong chat

        let last_id = first_id + count as u32 - 1;  // inclusive. Can't undeflow

        if s.min_id > s.max_id {  // initial fetch
            // handle initial fetch separately; skip all checks
            for msg in buf.chunks_exact(MSG_OUT_SIZE) {
                let msg = Message::new(msg.try_into().unwrap(), &s.chat_key)?;
                // println!("{}", msg);
                s.queue.push_back(msg);
            }
            s.min_id = first_id;
            s.max_id = last_id;
            continue;  // initial fetch finished, move to next packet
        }

        if s.max_id + 1 < first_id ||  // disconnected ahead
           s.min_id > last_id + 1 ||  // disconnected behind
           (s.min_id <= first_id && last_id <= s.max_id) ||  // already have this
           (first_id < s.min_id && s.max_id < last_id)  // overflow on both sides
        { continue }  // skip all these

        if forward {
            if last_id > s.max_id {  // good proper data here
                let i = if first_id <= s.max_id {s.max_id-first_id+1} else {0};
                assert_eq!(s.max_id + 1, first_id + i);
                for msg in buf.chunks_exact(MSG_OUT_SIZE).skip(i as usize) {
                    let msg = Message::new(msg.try_into().unwrap(), &s.chat_key)?;
                    // println!("{}", msg);
                    s.queue.push_back(msg);
                }
                // buf.chunks_exact(MSG_OUT_SIZE)
                //     .skip(i as usize)
                //     .map(|msg| Message::new(msg.try_into().unwrap(), &s.chat_key)?)
                //     .for_each(|msg| s.queue.push_back(msg));
                s.max_id = last_id;
            } else {  // points forwards but behind our data
                continue;
            }
        } else {  // not forwards (for scrolling up)
            todo!()
        }
    }
}

fn drawer(
    state: Arc<Mutex<GlobalState>>,
    msg_tx: std::sync::mpsc::Sender<String>,
)-> crossterm::Result<()> {
    use crossterm::{
        QueueableCommand,
        cursor,
        terminal::{
            self,
            ClearType,
            EnterAlternateScreen,
            LeaveAlternateScreen,
        },
        style::{
            style,
            SetAttribute,
            Attribute,
            PrintStyledContent,
            StyledContent,
            Color,
        },
        event::{
            MouseEvent,
        }
    };

    const BG_COLOUR: Color = Color::Rgb{r: 0xd0, g: 0xd0, b: 0xd0};
    const FG_COLOUR: Color = Color::Rgb{r: 0x66, g: 0x00, b: 0x33};

    enum ViewPos {
        Last,  // "most recent message on bottom"
        Index{msg_id: u16, chr_id: u8},  // id of TOP message, index of its first char
    }

    println!("Entering alternate screen...");
    let mut stdout = std::io::stdout();
    terminal::enable_raw_mode()?;
    stdout.queue(event::EnableMouseCapture)?;
    stdout.queue(EnterAlternateScreen)?;
    stdout.queue(terminal::Clear(ClearType::All))?;
    stdout.queue(cursor::DisableBlinking)?;
    stdout.queue(cursor::Hide)?;
    stdout.flush()?;

    fn draw_header(size: (u16, u16)) -> crossterm::Result<()> {
        let (w, h) = size;

        let mut stdout = std::io::stdout();
        stdout.queue(cursor::MoveTo(0, 0))?;

        let chat_name = "ChatName";  // TODO: get chat name in here
        let header_text = {
            let title = format!("> PubliChat {chat_name} <");
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

    fn draw_footer(cur_input: &str) -> crossterm::Result<()> {
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
        let text = style(cur_input)
            .with(FG_COLOUR)
            .on(BG_COLOUR);
        stdout.queue(PrintStyledContent(text))?;
        let spaces = style(" ".repeat(w as usize - 2 - cur_input.len()))
            .on(BG_COLOUR);
        stdout.queue(PrintStyledContent(spaces))?;
        stdout.flush()
    }

    fn draw_messages(
        view: &ViewPos,
        size: &(u16, u16),
        state: &Arc<Mutex<GlobalState>>,
    ) -> crossterm::Result<()> {
        // SIDE EFFECT: DELETES FOOTER!!!
        let state = state.lock().unwrap();  // TODO get rid of this unwrap!!!
        if state.queue.is_empty() {return Ok(())}

        let mut stdout = std::io::stdout();

        let (w, h) = size;
        let mut remaining_lines = h - 4;  // two lines used on top, two on bottom

        // TODO: find a way of changing backgroud nicely
        // stdout.queue(SetForegroundColor(Color::Black))?;
        // stdout.queue(SetBackgroundColor(Color::Grey))?;

        match view {
            ViewPos::Index { msg_id, chr_id } => {
                stdout.queue(cursor::MoveTo(0, 2))?;  // TODO: terminal too small?
                stdout.queue(terminal::Clear(ClearType::FromCursorDown))?;

                // TODO: print start.msg partial
                for msg in state.queue.range(1+usize::from(*msg_id)..) {
                    if remaining_lines == 0 { break }

                    let msg_str = msg.length + 27;  // TODO: magic number, length of message prefix
                    let line_count = (u16::from(msg_str) / w) + 1;
                    if let Some(res) = remaining_lines.checked_sub(line_count) {
                        // normal situation, whole message fits on screen
                        write!(stdout, "{msg}\r\n")?;
                        remaining_lines = res;
                    } else {
                        let printable_chars = remaining_lines * size.0;
                        write!(stdout, "{}", &msg.to_string()[..usize::from(printable_chars)])?;
                        
                        break;
                    }
                }
            }
            ViewPos::Last => {
                todo!()
            }
        }

        stdout.flush()
    }

    // let mut cur_pos = ViewPos::Last;  // TODO: should start with this
    let mut cur_pos = ViewPos::Index{msg_id: 0, chr_id: 0};
    fn move_pos(pos: &mut ViewPos, up: bool) {
        // positive is scolling up
        *pos = match pos {
            ViewPos::Last => ViewPos::Last,
            ViewPos::Index{msg_id, chr_id} => if up {
                ViewPos::Index{msg_id: *msg_id-1, chr_id: *chr_id}
            } else {
                // TODO: possible ViewPos::Last
                ViewPos::Index{msg_id: *msg_id+1, chr_id: *chr_id}
            },
        };
    }

    // draw first frame manually
    let mut cur_size = terminal::size()?;
    stdout.execute(Clear(ClearType::All))?;
    draw_header(cur_size)?;
    draw_messages(&cur_pos, &cur_size, &state)?;
    draw_footer("")?;

    let mut disp_str = String::with_capacity(50);

    let mut last_update = std::time::UNIX_EPOCH;

    // mainloop
    loop {
        // wait for events to come in. Update instantly if they do, otherwise
        // update at defined FPS when a new message comes in.
        match event::poll(_DISP_DELAY) {
            Ok(true) => match event::read()? {
                Event::Key(KeyEvent{code, modifiers}) => {
                    use crossterm::event::KeyCode::*;
                    use crossterm::event::KeyModifiers as Mod;
                    match (modifiers, code) {
                        (Mod::CONTROL, Char('c')) | (Mod::NONE, Esc) => break,
                        (Mod::NONE, Char(c)) | (Mod::SHIFT, Char(c)) => {
                            disp_str.push(c);
                            draw_footer(&disp_str)?;
                        },  // type c in msg
                        (Mod::NONE, Enter) => {
                            msg_tx.send(mem::take(&mut disp_str));
                            draw_footer("")?;
                        },  // send message
                        (Mod::NONE, Backspace) => {
                            disp_str.pop();
                            draw_footer(&disp_str)?;
                        },  // remove char
                        (Mod::CONTROL, Backspace) => {}  // remove word
                        (Mod::NONE, Delete) => {}  // remove char
                        (Mod::CONTROL, Delete) => {}  // remove word
                        (Mod::NONE, Up) => {
                            move_pos(&mut cur_pos, true);
                            draw_messages(&cur_pos, &cur_size, &state)?;
                            draw_footer(&disp_str)?;
                        },  // scroll up
                        (Mod::NONE, Down) => {
                            move_pos(&mut cur_pos, false);
                            draw_messages(&cur_pos, &cur_size, &state)?;
                            draw_footer(&disp_str)?;
                        },  // scroll down
                        (Mod::NONE, PageUp) => {}  // scroll way up
                        (Mod::NONE, PageDown) => {}  // scroll way down
                        (Mod::NONE, Home) => {}  // scroll way way up
                        (Mod::NONE, End) => {}  // scroll way way down
                        _ => continue,
                    }
                },
                Event::Mouse(MouseEvent{kind, ..}) => {
                    use crossterm::event::MouseEventKind;
                    match kind {
                        MouseEventKind::ScrollUp => move_pos(&mut cur_pos, true),
                        MouseEventKind::ScrollDown => move_pos(&mut cur_pos, false),
                        _ => continue,
                    }
                },
                Event::Resize(x, y) => {
                    cur_size = (x, y);
                    draw_header(cur_size)?;
                    draw_messages(&cur_pos, &cur_size, &state)?;
                    draw_footer(&disp_str)?;
                },
            },
            Ok(false) => {},  // No events to be processed
            Err(e) => break,
        }

        // re-draw all messages from time to time
        // this will display new messages as they come in
        if time::SystemTime::now().duration_since(last_update).unwrap() > _DISP_DELAY {
            draw_messages(&cur_pos, &cur_size, &state)?;
            draw_footer(&disp_str)?;
            last_update = time::SystemTime::now();
        }

    }

    stdout.queue(cursor::Show)?;
    stdout.queue(cursor::EnableBlinking)?;
    stdout.queue(LeaveAlternateScreen)?;
    stdout.queue(SetAttribute(Attribute::Reset))?;
    stdout.queue(event::DisableMouseCapture)?;
    stdout.flush()?;
    terminal::disable_raw_mode()?;
    println!("Exited alternate screen");
    Ok(())
}

fn requester(
    mut stream: TcpStream,
    state: Arc<Mutex<GlobalState>>,
    msg_rx: std::sync::mpsc::Receiver<String>,
) -> Res {
    let user_id = state.lock().map_err(|_| "Failed to lock state")?.user_id;
    let chat_id = state.lock().map_err(|_| "Failed to lock state")?.chat_id;
    let chat_key = state.lock().map_err(|_| "Failed to lock state")?.chat_key;
    let mut cypher_buf = [0; CYPHER_SIZE];
    while state.lock().map_err(|_| "Failed to lock state")?.queue.is_empty() {
        comm::send_fetch(&mut stream, &chat_id)?;
        if let Ok(msg) = msg_rx.try_recv() {
            cypher_buf = Message::make_cypher(&msg, &chat_key).unwrap();  // TODO: unwrap
            comm::send_msg(&mut stream, &chat_id, &user_id, &cypher_buf)?;
        }
        thread::sleep(FQ_DELAY);
    }
    loop {
        comm::send_query(
            &mut stream,
            &chat_id,
            true,
            50,
            state.lock().unwrap().max_id,
        )?;
        if let Ok(msg) = msg_rx.try_recv() {
            cypher_buf = Message::make_cypher(&msg, &chat_key).unwrap();  // TODO: unwrap
            comm::send_msg(&mut stream, &chat_id, &user_id, &cypher_buf)?;
        }
        thread::sleep(FQ_DELAY);
    }
}

fn main() -> Result<(), Box<dyn Error>> {  // TODO: return Res instead?
    println!("Starting client...");
    // arguments: addr:port title user

    let mut args = std::env::args().skip(1).collect::<Vec<_>>();

    let server_addr = args.get(0).ok_or("No addr given")?
        .to_socket_addrs()?
        .next().ok_or("Zero addrs received?")?;

    let chat = mem::take(args.get_mut(1).ok_or("No title given")?);
    let (chat_key, chat_id) = crypt::hash_twice(chat.as_bytes());

    let user = mem::take(args.get_mut(2).ok_or("No username given")?);
    let user_id = crypt::hash(user.as_bytes());

    println!("Connecting to server {:?}...", server_addr);
    let mut stream = TcpStream::connect(server_addr)?;
    println!("Connected!");

    stream.write_all(b"SMRT")?;

    let queue = VecDeque::with_capacity(500);
    let state = GlobalState {
        queue,
        chat_key,
        chat_id,
        user_id,
        min_id: 1,
        max_id: 0,
    };
    let state = Arc::new(Mutex::new(state));

    let (rx, tx) = std::sync::mpsc::channel::<String>();

    // start listener thread
    let stream2 = stream.try_clone()?;
    let state2 = state.clone();
    thread::spawn(|| {
        println!("Starting listener thread.");
        match listener(stream2, state2) {
            Ok(_) => println!("Listener thread finished"),
            Err(e) => println!("Listener thread crashed: {e}"),
        }
    });

    // start drawer thread
    let state3 = state.clone();
    thread::spawn(|| {
        println!("Starting drawer thread.");
        match drawer(state3, rx) {  // drawer recieves text input and send to requester
            Ok(_) => println!("Drawer thread finished"),
            Err(e) => println!("Drawer thread crashed: {e}"),
            // TODO: end process if one thread crashes?
        }
    });

    // main thread is requester thread
    println!("Starting requests");
    match requester(stream, state, tx) {  // requester sends messages from tx to server
        Ok(_) => println!("Request loop finished"),
        Err(e) => println!("Request loop crashed: {e}"),
    };

    Ok(())
}
