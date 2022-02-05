//! This crate implements the client of the remote counting service.
use remoc::prelude::*;
use std::{net::Ipv4Addr, time::Duration};
use tokio::net::TcpStream;

use counter::{Counter, CounterClient, TCP_PORT};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "test_server", about = "A server that clients can make requests to perform multiple Bluetooth tests")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long, help="Show additional information for troubleshooting such as details about the adapters")]
    debug: bool,
 
    // short and long flags (-a, --advertiser) will be deduced from the field's name     
    #[structopt(short, long, required=true, help="The Bluetooth controller address to use in the form XX:XX:XX:XX:XX:XX  ex: 5C:F3:70:A1:71:0F")]
    bluetooth_address: String,
}

#[tokio::main]
async fn main() {
    // Initialize logging.
    tracing_subscriber::FmtSubscriber::builder().init();
    let opt = Opt::from_args();

    println!("{:?}", opt);

    let debug_mode = opt.debug;    
    if debug_mode
    {
        println!("{:?}", opt);
    }
    let my_address = opt.bluetooth_address;
 
    let session = bluer::Session::new().await.unwrap();
  
    let adapter_names = session.adapter_names().await.unwrap();
    let adapter_name = adapter_names.first().expect("No Bluetooth adapter present");
    let mut adapter = session.adapter(adapter_name).unwrap();
    for adapter_name in adapter_names {
        println!("Checking Bluetooth adapter {}:", &adapter_name);
        let adapter_tmp = session.adapter(&adapter_name).unwrap();
        let address = adapter_tmp.address().await.unwrap();
        if  address.to_string() == my_address {
            adapter =  adapter_tmp;
            break;
        }
    };

    //let adapter_name = adapter_names.first().expect("No Bluetooth adapter present");
    //let adapter = session.adapter(adapter_name)?;
    let adapter_name = adapter.name();    
    adapter.set_powered(true).await.unwrap();
    if debug_mode
    {
        println!("Advertising on Bluetooth adapter {}", &adapter_name);
        println!("    Address:                    {}", adapter.address().await.unwrap());
        println!("    Address type:               {}", adapter.address_type().await.unwrap());
        println!("    Friendly name:              {}", adapter.alias().await.unwrap());
        println!("    System name:                {}", adapter.system_name().await.unwrap());
        println!("    Modalias:                   {:?}", adapter.modalias().await.unwrap());
        println!("    Powered:                    {:?}", adapter.is_powered().await.unwrap());        
    }
    // Establish TCP connection to server.
    let socket = TcpStream::connect((Ipv4Addr::LOCALHOST, TCP_PORT)).await.unwrap();
    let (socket_rx, socket_tx) = socket.into_split();

    // Establish a Remoc connection with default configuration over the TCP connection and
    // consume (i.e. receive) the counter client from the server.
    let mut client: CounterClient =
        remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx).consume().await.unwrap();

    // Subscribe to the counter watch and print each value change.
    println!("Subscribing to counter change notifications");
    let mut watch_rx = client.watch().await.unwrap();
    let watch_task = tokio::spawn(async move {
        loop {
            while let Ok(()) = watch_rx.changed().await {
                let value = watch_rx.borrow_and_update().unwrap();
                println!("Counter change notification: {}", *value);
            }
        }
    });
    println!("Done!");

    // Print current value.
    let value = client.value().await.unwrap();
    println!("Current counter value is {}\n", value);

    // Increase counter value.
    println!("Increasing counter value by 5");
    client.increase(5).await.unwrap();
    println!("Done!\n");

    // Print new value.
    let value = client.value().await.unwrap();
    println!("New counter value is {}\n", value);

    // Let the server count to the current value.
    println!("Asking the server to count to the current value with a step delay of 300ms...");
    let mut rx = client.count_to_value(1, Duration::from_millis(300)).await.unwrap();
    while let Ok(Some(i)) = rx.recv().await {
        println!("Server counts {}", i);
    }
    println!("Server is done counting.\n");

    // Wait for watch task.
    println!("Server exercise is done.");
    println!("You can now press Ctrl+C to exit or continue watching for value ");
    println!("change notification caused by other clients.");
    watch_task.await.unwrap();
}