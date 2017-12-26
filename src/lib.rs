extern crate bytes;

use bytes::BufMut;

#[derive(Copy, Clone)]
pub enum ServerType {
    Dedicated,
    NonDedicated,
    SourceTVRelay
}

#[derive(Copy, Clone)]
pub enum OS { 
    Linux,
    Windows,
    Mac
}

#[derive(Clone)]
pub struct ServerInfo {
    pub protocol_version: u8,
    pub name: String,
    pub map: String,
    pub folder: String,
    pub game: String,
    pub steamapp_id: i16,
    pub players: u8,
    pub max_players: u8,
    pub bots: u8,
    pub server_type: ServerType,
    pub os: OS,
    pub is_public: bool,
    pub uses_vac: bool,
    pub version: String,
    pub port: Option<i16>,
    pub steam_id: Option<u64>,
    pub spectator_port: Option<i16>,
    pub spectator_name: Option<String>,
    pub keywords: Option<String>,
    pub game_id: Option<u64>,
}

use bytes::{Buf, LittleEndian};
use std::io::Cursor;

fn get_string(cur: &mut Cursor<&mut Vec<u8>>) -> String {
    let mut s = String::with_capacity(64);
    loop {
        let b = cur.get_u8();
        if b == 0 {
            break
        } else {
            s.push(b as char);
        }
    }
    s
}

use std::io::{Error, ErrorKind, Result};
impl ServerInfo {
    fn from_bytes(b: &mut Vec<u8>) -> Result<ServerInfo> {
        let mut cur = Cursor::new(b);

        let header = cur.get_u8();
        if header != 0x49 {
            return Err(Error::new(ErrorKind::InvalidData, format!("Expected header `l` got `{}`", header as char)));
        }
        let protocol_version = cur.get_u8();
        let name = get_string(&mut cur);
        let map = get_string(&mut cur);
        let folder = get_string(&mut cur);
        let game = get_string(&mut cur);
        let steamapp_id = cur.get_i16::<LittleEndian>();
        let players = cur.get_u8();
        let max_players = cur.get_u8();
        let bots = cur.get_u8();
        let server_type = match cur.get_u8() as char {
            'd' => ServerType::Dedicated,
            'l' => ServerType::NonDedicated,
            'p' => ServerType::SourceTVRelay,
            c   => return Err(Error::new(ErrorKind::InvalidData, format!("Unknown server type: {}", c))),
        };
        let os = match cur.get_u8() as char {
            'l' => OS::Linux,
            'w' => OS::Windows,
            'm' => OS::Mac,
            c   => return Err(Error::new(ErrorKind::InvalidData, format!("Unknown environment: {}", c))),
        };
        let is_public = cur.get_u8() == 0;
        let uses_vac = cur.get_u8() == 1;
        let version = get_string(&mut cur);
        let edf = cur.get_u8();

        let port = if edf & 0x80 != 0 {
            Some(cur.get_i16::<LittleEndian>())
        } else {
            None
        };

        let steam_id = if edf & 0x10 != 0 {
            Some(cur.get_u64::<LittleEndian>())
        } else {
            None
        };

        let (spectator_port, spectator_name) = if edf & 0x40 != 0 {
            (Some(cur.get_i16::<LittleEndian>()),
             Some(get_string(&mut cur)))
        } else {
            (None, None)
        };

        let keywords = if edf & 0x20 != 0 {
            Some(get_string(&mut cur))
        } else {
            None
        };

        let game_id = if edf & 0x01 != 0 {
            Some(cur.get_u64::<LittleEndian>())
        } else {
            None
        };

        Ok(ServerInfo {
            protocol_version,
            name,
            map,
            folder,
            game,
            steamapp_id,
            players,
            max_players,
            bots,
            server_type,
            os,
            is_public,
            uses_vac,
            version,
            port,
            steam_id,
            spectator_port,
            spectator_name,
            keywords,
            game_id,
        })
    }
}

use std::net::{ToSocketAddrs, UdpSocket};
use std::io;
use std::time::Duration;

pub fn query<T: ToSocketAddrs>(addr: T) -> io::Result<ServerInfo> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    socket.connect(addr)?;

    let mut buf = vec![];

    buf.put_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0x54]);
    buf.put_slice(b"Source Engine Query\0");

    socket.send(&buf)?;

    let mut recbuf = vec![0; 1024];
    let rec = socket.recv(&mut recbuf)?;

    use bytes::{Buf, LittleEndian};
    use std::io::Cursor;

    let mut cur = Cursor::new(&recbuf[..rec]);
    let header = cur.get_i32::<LittleEndian>();

    let mut buf = if header == -1 {
        cur.bytes().to_owned()
    } else {
        return Err(Error::new(ErrorKind::InvalidData, format!("Unknown header: {}", header)));
    };

    ServerInfo::from_bytes(&mut buf)
}


