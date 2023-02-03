#![allow(unused)]
#![feature(drain_filter)]
extern crate midir;

// CORE MIDI TEST
use coremidi::{Destinations, Endpoint, Sources};

use std::sync::Mutex;
use std::sync::RwLock; // Potentially use this instead of Mutex for MidiInput and MidiOutput
use std::result::Result;

// PLAY TEST
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use std::io::{stdin, stdout, Write};


// Send messages to Erlang PID
// use tokio::sync::mpsc::{channel, Receiver, Sender};

use midir::{MidiInput, MidiOutput, MidiInputConnection, MidiOutputConnection, MidiInputPort, MidiOutputPort, Ignore, InitError};

use midir::os::unix::{VirtualInput, VirtualOutput};


use rustler::{Atom, Env, Error, NifResult, NifStruct, NifMap, NifTuple, ResourceArc, Term, Binary, OwnedBinary};



// This version uses threadlocal to create the MidiInput and MidiOutput objects
thread_local!(static GLOBAL_MIDI_INPUT_RESULT: Result<MidiInput, InitError> = MidiInput::new("MIDIex"));
thread_local!(static GLOBAL_MIDI_OUTPUT_RESULT: Result<MidiOutput, InitError> = MidiOutput::new("MIDIex"));



// Atoms
mod atoms {
    rustler::atoms! {
        ok,
        error,

        input,
        output,

        message
    }
}





// Send message to Erlang
#[rustler::nif]
pub fn subscribe(env: Env) -> Atom {  

    let pid = env.pid();

    let mut the_message = Term::map_new(env);

    the_message = the_message.map_put("message", "You recieved this message from Rust.").unwrap();

    env.send(&pid.clone(), the_message);

    sleep(Duration::from_millis(1000));    

    atoms::ok()
}


// fn poll(env: Env, resource: ResourceArc<Ref>) -> (Atom, ResourceArc<Ref>) {
//     send(resource.clone(), Msg::Poll(env.pid()));

//     (ok(), resource)
// }




// fn deliver(env: Env, resource: ResourceArc<Ref>, msg: Message) -> (Atom, ResourceArc<Ref>) {
//     send(resource.clone(), Msg::Send(env.pid(), msg));
//     (ok(), resource)
// }


#[rustler::nif(schedule = "DirtyCpu")]
fn listen(midi_port: MidiPort) -> Result<Vec<u8>, Error> {

    // let mut data: Binary = Binary::new();

    let mut msg: Vec<u8> = Vec::new();
  
    if midi_port.direction == atoms::input()  {

        println!("\rIs an input port");

        if let MidiexMidiPortRef::Input(in_port) = &midi_port.port_ref.0 { 
            println!("\r\tIn connections section");
        
                let mut midi_input = MidiInput::new("MIDIex").expect("Midi input"); 

                let port_name = midi_input.port_name(in_port).unwrap();

                // println!("\r\tPort is called {:?}", port_name);


                // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
                let _conn_in = midi_input.connect(&in_port, "MIDIex input port", move |stamp, message, msg| {
                     
                    println!("\n{}: {:?} (len = {})\r", stamp, message, message.len());

                    // let mut bin_msg = OwnedBinary::new(message.len()).expect("Owned binary message created");
                    let mut bin_msg =  message.to_vec();


                   
                    


                    *msg = bin_msg;

                }, (msg).clone());   

                sleep(Duration::from_millis(1000));    
                
         
          
        };

    } else {
        println!("\nCannot listen to an output port - use the connect/1 function instead");
        // return atoms::error()
        return Err(Error::RaiseTerm(Box::new(
            "Not an input port.".to_string(),
        ))) 
    }

    Ok(msg)
}



#[rustler::nif]
fn connect(midi_port: MidiPort) -> Result<OutConn, Error>{

    if midi_port.direction == atoms::output()  {
        println!("OUTPUT");

        let mut midi_output = MidiOutput::new("MIDIex").expect("Midi output");  

        // let mut port_ref = midi_port.port_ref.0;

        if let MidiexMidiPortRef::Output(port) = &midi_port.port_ref.0 { 

            println!("OUTPUT PORT");

            let mut conn_out_result = midi_output.connect(&port, "MIDIex");

            let mut conn_out = match conn_out_result {
                Ok(conn_out) => {
                    println!("CONNECTION MADE");
                    
                    return Ok(
                        OutConn {
                            conn_ref: ResourceArc::new(OutConnRef::new(conn_out)),
                            // midi_port: midi_port,   
                            name: midi_port.name,
                            port_num: midi_port.num,
                        }
                    )

                },
                Err(error) => panic!("Midi Output Connection Error: Problem getting midi output connection. Error: {:?}", error)
            };
    
            

         };


    } else {
        println!("INPUT");

        let mut midi_input = MidiInput::new("MIDIex").expect("Midi output");  

        // let mut port_ref = midi_port.port_ref.0;

        if let MidiexMidiPortRef::Input(port) = &midi_port.port_ref.0 { 

            println!("INPUT PORT");

            // let mut conn_in_result = midi_input.connect(&port, "MIDIex");

            // let mut conn_in = match conn_in_result {
            //     Ok(conn_in) => { println!("CONNECTION MADE"); conn_in},
            //     Err(error) => panic!("Midi Input Connection Error: Problem getting midi input connection. Error: {:?}", error)
            // };

            return Err(Error::RaiseTerm(Box::new(
                "Input connection rather than output.".to_string(),
            )))
    
            

         };
    }



    Err(Error::RaiseTerm(Box::new(
        "No output connection".to_string(),
    )))
}


#[rustler::nif]
fn create_virtual_output(name: String) -> Result<OutConn, Error>{

    let mut midi_output = MidiOutput::new("MIDIex").expect("Midi output");
    let mut midi_input = MidiInput::new("MIDIex").expect("Midi input");
    midi_input.ignore(Ignore::None);

    let mut conn_out = midi_output.create_virtual(&name).expect("Midi MidiOutputConnection");

    // Even though we've created an output port, beacause it's a virtual port it is listed as an 'input' when querying the OS for available devices. 
    let port_index = midi_input.port_count();

    // Just in case added port_ref vack into OutConn
    // let new_port: MidiInputPort = midi_input.ports().into_iter().rev().next().unwrap();

    return Ok(
        OutConn {
            conn_ref: ResourceArc::new(OutConnRef::new(conn_out)),
            name: name,
            port_num: port_index-1,

            // Just in case port_ref is added back in:
            // midi_port: MidiPort{
            //     direction: atoms::output(),
            //     name: name,
            //     num: port_index,
            //     port_ref: ResourceArc::new(FlexiPort::new(MidiexMidiPortRef::Output(MidiOutputPort::clone(&new_port)))) 
            // }   
        }
    )

}


#[rustler::nif(schedule = "DirtyCpu")]
fn send_msg(midi_out_conn: OutConn, message: Binary) -> Result<OutConn, Error>{

    println!("Message recieved");

    let mut midi_output = MidiOutput::new("MIDIex").expect("Midi output"); 

    {
        let mut conn_out = midi_out_conn.conn_ref.0.lock().unwrap();
        conn_out.send(&message);
    }
    
    Ok(midi_out_conn)
}





// MIDI Connection

// pub enum MidiexConnRef {
//     Input(MidiInputConnection), 
//     Output(MidiOutputConnection), 
//   }


// pub struct FlexiConn(pub MidiexConnRef);

// impl FlexiConn {
//     pub fn new(data: MidiexConnRef) -> Self {
//         Self(data)
//     }
// }
  

#[derive(NifStruct)]
#[module = "Midiex.OutConn"]
pub struct OutConn {
    conn_ref: ResourceArc<OutConnRef>,
    // midi_port: MidiPort,
    name: String,
    port_num: usize,
}

pub struct OutConnRef(pub Mutex<MidiOutputConnection>);

impl OutConnRef {
    pub fn new(data: MidiOutputConnection) -> Self {
        Self(Mutex::new(data))
    }
}



// ==========
// MIDI Ports
// ==========
pub enum MidiexMidiPortRef {
    Input(MidiInputPort), 
    Output(MidiOutputPort), 
  }

pub struct FlexiPort(pub MidiexMidiPortRef);

impl FlexiPort {
    pub fn new(data: MidiexMidiPortRef) -> Self {
        Self(data)
    }
}

#[derive(NifStruct)]
#[module = "Midiex.MidiPort"]
pub struct MidiPort {
    direction: Atom,
    name: String,
    num: usize,
    port_ref: ResourceArc<FlexiPort>,
}

#[derive(NifMap)]
pub struct NumPorts {
    input: usize,
    output: usize 
}



// MIDI IO related 

pub struct MidiexMidiInputRef(pub Mutex<MidiInput>);
pub struct MidiexMidiOutputRef(pub Mutex<MidiOutput>);

impl MidiexMidiInputRef {
    pub fn new(data: MidiInput) -> Self {
        Self(Mutex::new(data))
    }
}

impl MidiexMidiOutputRef {
    pub fn new(data: MidiOutput) -> Self {
        Self(Mutex::new(data))
    }
}


#[derive(NifStruct)]
#[module = "Midiex.MidiIO"]
pub struct MidiexMidiIO {
    pub resource_input: ResourceArc<MidiexMidiInputRef>,
    pub resource_output: ResourceArc<MidiexMidiOutputRef>,
    pub active_connections: Vec<MidiPort>,
}

impl MidiexMidiIO {
    pub fn new(midi_input: MidiInput, midi_output: MidiOutput) -> Self {
        Self {
            resource_input: ResourceArc::new(MidiexMidiInputRef::new(midi_input)),
            resource_output: ResourceArc::new(MidiexMidiOutputRef::new(midi_output)),
            active_connections: Vec::new()
        }
    }

}




// List all the ports, taking midi_io as input
#[rustler::nif(schedule = "DirtyCpu")]
fn list_ports() -> Result<Vec<MidiPort>, Error> {

    let mut vec_of_devices: Vec<MidiPort> = Vec::new();

    GLOBAL_MIDI_INPUT_RESULT.with(|midi_input_result| {

        let midi_input = match midi_input_result {
            Ok(midi_device) => midi_device,
            Err(error) => panic!("Problem getting midi input devices. Error: {:?}", error)
        };

        println!("\nMidi input ports: {:?}\n\r", midi_input.port_count());

        for (i, p) in midi_input.ports().iter().enumerate() {
        
            let port_name = if let Ok(port_name) = midi_input.port_name(&p) { port_name } else { "No device name given".to_string() };
    
                vec_of_devices.push(
                    MidiPort{
                        direction: atoms::input(),
                        name: port_name,
                        num: i,
                        port_ref: ResourceArc::new(FlexiPort::new(MidiexMidiPortRef::Input(MidiInputPort::clone(p))))         
                    });
        
        }
    
    });

    GLOBAL_MIDI_OUTPUT_RESULT.with(|midi_output_result| {

        let midi_output = match midi_output_result {
            Ok(midi_device) => midi_device,
            Err(error) => panic!("Problem getting midi output devices. Error: {:?}", error)
        };

        println!("Midi output ports: {:?}\n\r", midi_output.port_count());

        for (i, p) in midi_output.ports().iter().enumerate() {  
        
            let port_name = if let Ok(port_name) = midi_output.port_name(&p) { port_name } else { "No device name given".to_string() };
          
                vec_of_devices.push(
                    MidiPort{
                        direction: atoms::output(),
                        name: port_name,
                        num: i,
                        port_ref: ResourceArc::new(FlexiPort::new(MidiexMidiPortRef::Output(MidiOutputPort::clone(p))))
                    });
    
        }

    });

    return Ok(vec_of_devices)

}


#[rustler::nif(schedule = "DirtyCpu")]
fn count_ports() -> Result<NumPorts, Error>{

    let mut num_input_ports = 0;
    let mut num_output_ports = 0;

    GLOBAL_MIDI_INPUT_RESULT.with(|midi_input_result| {

        let midi_input = match midi_input_result {
            Ok(midi_device) => midi_device,
            Err(error) => panic!("Problem getting midi input devices. Error: {:?}", error)
        };

        num_input_ports = midi_input.port_count();

    });

    GLOBAL_MIDI_OUTPUT_RESULT.with(|midi_output_result| {

        let midi_output = match midi_output_result {
            Ok(midi_device) => midi_device,
            Err(error) => panic!("Problem getting midi output devices. Error: {:?}", error)
        };

        num_output_ports = midi_output.port_count();

    });

    return Ok( NumPorts { input: num_input_ports, output:  num_output_ports } )
}




fn on_load(env: Env, _info: Term) -> bool {
  
    // MIDI Input and Output object for the OS
    rustler::resource!(MidiexMidiInputRef, env);
    rustler::resource!(MidiexMidiOutputRef, env);
    
    // MIDI ports (both input and output)
    rustler::resource!(FlexiPort, env);
    rustler::resource!(MidiexMidiPortRef, env);

    // MIDI connection to a MIDI port
    // rustler::resource!(FlexiConn, env);
    // rustler::resource!(MidiexConnRef, env);
    rustler::resource!(OutConnRef, env);

    true
}

rustler::init!(
    "Elixir.Midiex",
    [count_ports, list_ports, connect, send_msg, subscribe, create_virtual_output, listen],
    load = on_load
);