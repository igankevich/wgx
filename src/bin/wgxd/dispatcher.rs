use std::time::Duration;
use std::time::SystemTime;

use log::error;
use mio::Events;
use mio::Poll;
use mio::Token;
use mio::Waker;
use static_assertions::const_assert;

use crate::format_error;
use crate::Config;
use crate::Error;
use crate::UnixServer;
use crate::WireguardRelay;

pub(crate) struct Dispatcher {
    poll: Poll,
    wg_relay: WireguardRelay,
    unix_server: UnixServer,
}

impl Dispatcher {
    pub(crate) fn new(config: Config) -> Result<Self, Error> {
        let mut poll = Poll::new()?;
        let unix_server = UnixServer::new(
            config.unix_socket_path.as_path(),
            UNIX_SERVER_TOKEN,
            &mut poll,
        )?;
        let wg_relay = WireguardRelay::new(config, UDP_SERVER_TOKEN, &mut poll)?;
        Ok(Self {
            poll,
            wg_relay,
            unix_server,
        })
    }

    pub(crate) fn waker(&self) -> Result<Waker, Error> {
        Ok(Waker::new(self.poll.registry(), WAKE_TOKEN)?)
    }

    pub(crate) fn run(mut self) -> Result<(), Error> {
        let mut events = Events::with_capacity(MAX_EVENTS);
        loop {
            events.clear();
            let timeout = self.wg_relay.next_event_time().map(|t| {
                t.duration_since(SystemTime::now())
                    .unwrap_or(Duration::ZERO)
            });
            match self.poll.poll(&mut events, timeout) {
                Ok(()) => Ok(()),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => Ok(()),
                other => other,
            }?;
            self.wg_relay.advance(SystemTime::now());
            for event in events.iter() {
                let ret = match event.token() {
                    WAKE_TOKEN => return Ok(()),
                    UDP_SERVER_TOKEN => {
                        if event.is_readable() {
                            self.wg_relay.on_event()
                        } else {
                            Ok(())
                        }
                    }
                    UNIX_SERVER_TOKEN => {
                        if event.is_readable() {
                            self.unix_server.on_server_event(
                                UNIX_TOKEN_MIN,
                                UNIX_TOKEN_MAX,
                                &mut self.poll,
                            )
                        } else {
                            Ok(())
                        }
                    }
                    Token(i) if (UNIX_TOKEN_MIN..=UNIX_TOKEN_MAX).contains(&i) => self
                        .unix_server
                        .on_client_event(event, &mut self.wg_relay, &mut self.poll),
                    Token(i) => Err(format_error!("unknown event {}", i)),
                };
                if let Err(e) = ret {
                    error!("dispatcher error: {}", e);
                }
            }
        }
    }
}

const MAX_EVENTS: usize = 1024;
const WAKE_TOKEN: Token = Token(usize::MAX);
const UDP_SERVER_TOKEN: Token = Token(1);
const UNIX_SERVER_TOKEN: Token = Token(2);
const MAX_UNIX_CLIENTS: usize = 1000;
const UNIX_TOKEN_MIN: usize = 1000;
const UNIX_TOKEN_MAX: usize = UNIX_TOKEN_MIN + MAX_UNIX_CLIENTS - 1;

const_assert!(UNIX_TOKEN_MIN <= UNIX_TOKEN_MAX);
const_assert!(MAX_UNIX_CLIENTS == UNIX_TOKEN_MAX - UNIX_TOKEN_MIN + 1);
