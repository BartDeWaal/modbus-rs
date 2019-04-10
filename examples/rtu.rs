extern crate modbus;
extern crate serialport;

use modbus::rtu::Connection;
use modbus::Client;

fn main() {
    if let Ok(port) = serialport::open("/dev/ttyUSB1") {
        let conn = Connection::new(port);
        let mut server = conn.get_server(1);
        let result = server.read_holding_registers(0, 2).unwrap();
        println!("Result holding register: {}", result[0]);
        let result = server.read_input_registers(0, 2).unwrap();
        println!("Result input register: {}", result[0]);

        println!("Write register value 44");
        server.write_single_register(0, 44);
        let result = server.read_holding_registers(0, 2).unwrap();
        println!("Result holding register: {}", result[0]);
        let result = server.read_input_registers(0, 2).unwrap();
        println!("Result input register: {}", result[0]);
    }
}
