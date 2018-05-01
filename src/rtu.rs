extern crate byteorder;
extern crate crc16;

use std::cell::RefCell;
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use self::crc16::{State, MODBUS};
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use {Client, Coil, Error, Result};

pub struct Connection<T>
where
    T: Read + Write + ?Sized,
{
    port: RefCell<Box<T>>,
    timeout: Duration,
}

impl<T> Connection<T>
where
    T: Read + Write + ?Sized,
{
    pub fn new(port: Box<T>) -> Connection<T> {
        Connection {
            port: RefCell::new(port),
            timeout: Duration::from_millis(500), // TODO: make setting
        }
    }

    fn write_with_crc(&self, msg: &[u8]) {
        let mut msg = msg.to_vec();

        //Calculate CRC
        let mut crc_bytes = [0, 0];
        LittleEndian::write_u16(&mut crc_bytes, State::<MODBUS>::calculate(&msg));
        msg.extend(crc_bytes.iter());

        if let Ok(mut port) = self.port.try_borrow_mut() {
            let _ = port.write(&msg);
        }
    }

    pub fn get_server<'a>(&'a self, id: u8) -> Server<'a, T> {
        Server {
            id: id,
            connection: self,
        }
    }

    fn read(
        &self,
        expected_bytes: Option<usize>,
        expected_id: u8,
        expected_function: u8,
    ) -> Result<Vec<u8>> {
        let mut result = self.read_dont_check_crc(expected_bytes, expected_id, expected_function)?;
        // TODO - check crc
        let _crc_byte2 = result.pop().unwrap();
        let _crc_byte1 = result.pop().unwrap();

        Ok(result)
    }

    // Perform a read with timeout, but don't check the CRC (that should be done by a different
    // function)
    fn read_dont_check_crc(
        &self,
        expected_bytes: Option<usize>,
        expected_id: u8,
        expected_function: u8,
    ) -> Result<Vec<u8>> {
        // Open the port
        let mut expected_bytes = expected_bytes;
        if let Ok(mut port) = self.port.try_borrow_mut() {
            let mut response = Vec::new();
            let start = Instant::now();

            // Keep reading until we have as many bytes as we expect, or the time runs out
            while start.elapsed() < self.timeout {
                let _num_bytes_read = port.read_to_end(&mut response);
                // Make sure we are getting the right ID
                if response.len() >= 1 && response[0] != expected_id {
                    return Err(Error::InvalidResponse);
                }
                if response.len() >= 2 && response[1] != expected_function {
                    // TODO: handle error responses
                    return Err(Error::InvalidResponse);
                }
                // check to see if we have enought bytes
                if let Some(eb) = expected_bytes {
                    if response.len() >= eb {
                        return Ok(response);
                    }
                    continue;
                } else {
                    // If the expected bytes are None, that means that this is a function that
                    // returns how many bytes it will return. This is always the third byte
                    if response.len() >= 3 {
                        expected_bytes = Some(
                            response[2] as usize + 5, // Add address, function, bytes and crc
                        );
                    }
                }
            }
            // Timeout
            Err(Error::InvalidResponse) // TODO: find better error
        } else {
            // Can't open the port
            Err(Error::InvalidResponse) // TODO: find better error
        }
    }
}

pub struct Server<'a, T>
where
    T: 'a + Read + Write + ?Sized,
{
    id: u8,
    connection: &'a Connection<T>,
}

impl<'a, T> Client for Server<'a, T>
where
    T: Read + Write + ?Sized,
{
    fn read_holding_registers(&mut self, address: u16, quantity: u16) -> Result<Vec<u16>> {
        let mut msg = Vec::new();
        msg.push(self.id);

        msg.push(0x03); // Read holding register

        let mut address_bytes = [0, 0];
        BigEndian::write_u16(&mut address_bytes, address);
        msg.extend(address_bytes.iter());

        let mut quantity_bytes = [0, 0];
        BigEndian::write_u16(&mut quantity_bytes, quantity);
        msg.extend(quantity_bytes.iter());

        self.connection.write_with_crc(&msg);

        let response = self.connection.read(None, self.id, 0x03)?;

        // Turn the response into data
        let mut data = response[3..].iter();
        let mut result = Vec::new();

        while let Some(byte1) = data.next() {
            if let Some(byte2) = data.next() {
                result.push((*byte1 as u16) * 0x0100 + *byte2 as u16);
            } else {
                // Not an even number of bytes!
                return Err(Error::InvalidResponse);
            }
        }

        Ok(result)
    }

    fn read_discrete_inputs(&mut self, _address: u16, _quantity: u16) -> Result<Vec<Coil>> {
        unimplemented!();
    }
    fn read_coils(&mut self, _address: u16, _quantity: u16) -> Result<Vec<Coil>> {
        unimplemented!();
    }
    fn write_single_coil(&mut self, _address: u16, _value: Coil) -> Result<()> {
        unimplemented!();
    }
    fn write_multiple_coils(&mut self, _address: u16, _coils: &[Coil]) -> Result<()> {
        unimplemented!();
    }
    fn read_input_registers(&mut self, _address: u16, _quantity: u16) -> Result<Vec<u16>> {
        unimplemented!();
    }
    fn write_single_register(&mut self, _address: u16, _value: u16) -> Result<()> {
        unimplemented!();
    }
    fn write_multiple_registers(&mut self, _address: u16, _values: &[u16]) -> Result<()> {
        unimplemented!();
    }
}
