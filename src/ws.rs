use std::net::TcpStream;
use std::collections::VecDeque;
use std::io::{self, Write, Read, Error, ErrorKind};

pub struct WsStream {
    tcp: TcpStream,
    data: VecDeque<u8>,
}

impl WsStream {
    pub fn new(tcp: TcpStream) -> Self {
        WsStream{ tcp, data: VecDeque::new() }
    }

    pub fn handshake(&mut self, _key_in: &[u8]) { todo!() }

    fn wrap(data: &[u8]) -> Option<Vec<u8>> {
        let len = data.len();
        let mut res: Vec<u8> = Vec::with_capacity(1 + 1 + 8 + len);

        // push starting header
        res.push(0b1_000_0010);  // header (fin=1; op=2(binary))

        // push payload length
        if len <= 125 {
            res.push(len as u8);  // can't fail
        } else if let Ok(len) = u16::try_from(len) {
            res.push(126);
            res.extend_from_slice(&len.to_be_bytes());
        } else if let Ok(len) = u64::try_from(len) {
            // happens if len > 65535 bytes. Should never happen in our case.
            res.push(127);
            res.extend_from_slice(&len.to_be_bytes());
        } else {
            return None;
        }

        // push the actual data
        res.extend_from_slice(data);
        Some(res)
    }

    fn parse_header(header: &[u8; 2]) -> Option<(bool, u8)> {
        // parses first two bytes of WS packet
        // returns None if header is invalid
        // otherwise returns length of packet and whether it's a ping

        let fin = header[0] & 0b1000_0000 != 0;
        // RSVn bits ignored
        let opc = header[0] & 0b0000_1111;
        let msk = header[1] & 0b1000_0000 != 0;
        let len = header[1] & 0b0111_1111;

        if !msk { return None }  // clients MUST mask
        if !fin && opc > 0x2 { return None }  // frag only defined for 0, 1, 2

        match opc {
            0xA => None,  // todo: all recieved pongs should be ignored
            0x9 => if len > 125 { None } else { Some((true, len)) },  // pings musn't be longer than 125
            _   => Some((false, len)),  // valid non-ping packet
        }
    }

    fn pong(&mut self, header: &[u8; 2]) -> Option<()> {
        // return a pong for a given ping
        // reads data from self.tcp, so after parsing the header
        // the tcp stream should be left untouched.
        // assumes mask is given.

        let len = header[1] & 0b0111_1111;
        assert!(len <= 125);
        let mut data = vec![0; (2 + len).into()];

        // add modified header
        data.copy_from_slice(header);
        data[0] ^= 0b11;  // flips last two bits op opcode (turn 0x9 into 0xA)

        // read remaining data into vec
        self.tcp.read_exact(&mut data[2..]).ok()?;
        
        // send it off
        self.tcp.write_all(&data).ok()?;
        self.tcp.flush().ok()?;

        Some(())
    }

    fn get_true_len(&mut self, len: u8) -> Option<usize> {
        // reads following bytes from tcp if needed
        match len {
            len if len <= 125 => Some(len.into()),
            126 => {
                let mut buf = [0; 2];
                self.tcp.read_exact(&mut buf).ok()?;
                Some(u16::from_be_bytes(buf).into())
            },
            127 => {
                let mut buf = [0; 8];
                self.tcp.read_exact(&mut buf).ok()?;
                Some(u64::from_be_bytes(buf).try_into().expect("can't unwrap u64 into usize"))
            },
            _ => None,  // bigger handled already
        }
    }

    fn decode(data: &mut [u8]) {
        // First four bytes are the mask. Xor the rest of the bytes with these
        // let mut mask = [0; 4];
        let mask = [
            data[0],
            data[1],
            data[2],
            data[3],
        ];

        data[4..].iter_mut()
            .zip(mask.iter().cycle())
            .for_each(|(byte, mask)| *byte ^= mask);
    }
}

impl Read for WsStream {
    fn read(&mut self, dest_buf: &mut [u8]) -> io::Result<usize> {
        // Reads some data to the buffer. IF reading from TCP is
        // needed, will read exactly ONE ws packet. Call read_exact
        // to fill buffer completely.

        while self.data.is_empty() {  // `if` would've probably been ok too
            // no data in the buffer: read from TCP

            let mut header_buf = [0; 2];
            self.tcp.read_exact(&mut header_buf)?;
    
            // get len byte from packet
            let len = loop {  // loop to get rid of all pings
                match Self::parse_header(&header_buf) {
                    None => {
                        return Err(Error::from(ErrorKind::Other))
                    },
                    Some((true, _)) => if self.pong(&header_buf).is_none() {
                        return Err(Error::from(ErrorKind::Other))
                    },
                    Some((false, len)) => break len,
                }
            };
    
            // convert len byte into actual length
            let len = match self.get_true_len(len) {
                Some(len) => len,
                None => return Err(Error::from(ErrorKind::Other))
            };
    
            // get data
            let mut recieved_data = vec![0; 4+len];
            self.tcp.read_exact(&mut recieved_data)?;

            // decode (first four bytes are mask)
            Self::decode(&mut recieved_data);
    
            // add to main buffer
            self.data.extend(&recieved_data[4..]);
        }

        // There's something in the queue - move it to the buffer
        let len = self.data.len().min(dest_buf.len());  // how many bytes can be filled in
        dest_buf[..len].fill_with(|| self.data.pop_front().unwrap());  // can't fail
        Ok(len)
    }
}

impl Write for WsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match Self::wrap(buf) {
            Some(data) => {
                self.tcp.write_all(&data)?;
                Ok(buf.len())
            },
            None => Err(Error::from(ErrorKind::Other)),
        }
    }

    fn flush(&mut self) -> io::Result<()> { self.tcp.flush() }
}
