extern crate modbus;
extern crate serialport;

use modbus::Client;
use modbus::rtu::Connection;

fn main() {
    if let Ok(port) = serialport::open("/dev/ttyUSB0") {
        let conn = Connection::new(port);
        let mut server = conn.get_server(1);
        let result = server.read_holding_registers(0, 2).unwrap();
        println!("Result: {}", result[0]);
    }
}
