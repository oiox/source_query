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
/// Information about a server returned by an A2FS_INFO request.
pub struct Response {
    /// Protocol version used by the server.
    pub protocol_version: u8,
    /// Name of the server.
    pub name: String,
    /// Currently running map of the server.
    pub map: String,
    /// Name of the folder containing the game files.
    pub folder: String,
    /// Full name of the game.
    pub game: String,
    /// Steam Application ID of the game.
    pub steamapp_id: i16,
    /// Number of players on the server.
    pub players: u8,
    /// Maximum number of players on the server.
    pub max_players: u8,
    /// Number of bots on the server.
    pub bots: u8,
    /// Type of the server.
    pub server_type: ServerType,
    /// Operating system of the server.
    pub os: OS,
    /// Indicates whether the server requires a password.
    pub is_public: bool,
    /// Indicates whether the server uses VAC.
    pub uses_vac: bool,
    /// Version of the game installed on the server.
    pub version: String,
    /// Server's game port number.
    pub port: Option<i16>,
    /// Server's Steam ID.
    pub steam_id: Option<u64>,
    /// Spectator port number for SourceTV.
    pub spectator_port: Option<i16>,
    /// Name of the spectator server for SourceTV.
    pub spectator_name: Option<String>,
    /// Tags that describe the game according to the server.
    pub keywords: Option<String>,
    /// Server's Game ID.
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
impl Response {
    fn from_bytes(b: &mut Vec<u8>) -> Result<Self> {
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

        Ok(Self {
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

/// Query a Source game server with the [Source Queries](https://developer.valvesoftware.com/wiki/Server_Queries) protocol using an [A2FS_INFO](https://developer.valvesoftware.com/wiki/Server_Queries#A2S_INFO) request.
///
/// Blocks the current thread till the request completed or the timeout was reached.
/// Returns `ServerInfo` on success with various informations about the server.
///
/// # Examples
///
/// Query a server with address 1.2.3.4 and port 27015 with no timeout.
///
/// ```
/// use source_query::info;
///
/// let info = info::query("1.2.3.4:27015", None)?;
/// ```
///
/// Query a server with address 1.2.3.4 and port 27015 with a timeout of 3 seconds.
///
/// ```
/// use source_query::info;
/// use std::time::Duration;
///
/// let info = info::query("1.2.3.4:27015", Some(Duration::from_secs(3)))?;
/// ```
pub fn query<T: ToSocketAddrs>(addr: T, timeout: Option<Duration>) -> io::Result<Response> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    socket.set_read_timeout(timeout)?;
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

    Response::from_bytes(&mut buf)
}


