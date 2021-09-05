use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error,
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::Duration,
};

use crate::state::Value;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum IncomingMessage {
    Test,
    State(HashMap<String, Value>),
    Disconnect,
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum OutgoingMessage {
    Test,
    Disconnect,
}

pub struct Incoming {
    receiver: Receiver<IncomingMessage>,
}

impl Iterator for Incoming {
    type Item = IncomingMessage;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let received = self.receiver.recv();
            match received {
                Ok(IncomingMessage::Disconnect) => return None,
                Ok(message) => {
                    log::trace!("Received message: {:?}", message);
                    return Some(message);
                }
                Err(e) => log::error!("Bad IPC message: {}", e),
            }
        }
    }
}

pub struct Outgoing {
    sender: Sender<OutgoingMessage>,
}

impl Outgoing {
    pub fn send(&self, message: OutgoingMessage) {
        if let Err(e) = self.sender.send(message) {
            log::error!("Failed to send IPC message: {}", e);
        }
    }
}

fn open_outgoing_stream(
    port: u16,
    receiver: &Receiver<OutgoingMessage>,
) -> Result<bool, Box<dyn Error>> {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port))?;
    log::trace!("IPC outgoing stream connected");

    for message in receiver.iter() {
        log::trace!("Sending message: {:?}", message);
        let mut data = serde_json::to_vec(&message)?;
        data.push(0x0a);
        stream.write_all(&data)?;

        match message {
            OutgoingMessage::Disconnect => {
                return Ok(false);
            }
            _ => stream.flush()?,
        }
    }

    Ok(true)
}

fn open_incoming_stream(
    port: u16,
    sender: Sender<IncomingMessage>,
) -> Result<bool, Box<dyn Error>> {
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port))?;
    log::trace!("IPC incoming stream connected");

    let reader = BufReader::new(stream);
    let lines = reader.lines();

    for line in lines {
        match serde_json::from_str(&line?)? {
            IncomingMessage::Disconnect => {
                sender.send(IncomingMessage::Disconnect)?;
                return Ok(false);
            }
            message => sender.send(message)?,
        }
    }

    Ok(true)
}

pub fn connect(incoming_port: u16, outgoing_port: u16) -> (Incoming, Outgoing) {
    let (incoming_sender, incoming_receiver) = channel();
    let (outgoing_sender, outgoing_receiver) = channel();

    thread::spawn(move || loop {
        match open_incoming_stream(incoming_port, incoming_sender.clone()) {
            Ok(false) => return,
            Ok(true) => (),
            Err(e) => log::error!("IPC incoming stream error: {}", e),
        }

        thread::sleep(Duration::from_millis(200));
    });

    thread::spawn(move || loop {
        match open_outgoing_stream(outgoing_port, &outgoing_receiver) {
            Ok(false) => return,
            Ok(true) => (),
            Err(e) => log::error!("IPC outgoing stream error: {}", e),
        }

        thread::sleep(Duration::from_millis(200));
    });

    (
        Incoming {
            receiver: incoming_receiver,
        },
        Outgoing {
            sender: outgoing_sender,
        },
    )
}