extern crate byteorder;
extern crate crc16;

use self::crc16::{State, MODBUS};
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use std::cell::RefCell;
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use {Client, Coil, Error, Function, Reason, Result};

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
            timeout: Duration::from_millis(5000), // TODO: make setting
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
        // Add two bytes to expected length for CRC
        let mut result = self.read_dont_check_crc(
            expected_bytes.map(|x| x + 2),
            expected_id,
            expected_function,
        )?;

        // first byte of crc, bitshifted
        let crc = result.pop().unwrap() as u16 * 0x0100;
        // second byte of crc
        let crc = crc + result.pop().unwrap() as u16;

        let correct_crc = State::<MODBUS>::calculate(&result);

        if crc != correct_crc {
            return Err(Error::IncorrectCRC);
        }

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
                    println!("Wrong Id");
                    return Err(Error::InvalidResponse);
                }
                if response.len() >= 2 && response[1] != expected_function {
                    println!(
                        "Wrong Function, got {}, expected {}",
                        response[1], expected_function
                    );
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
            Err(Error::TimeOut)
        } else {
            // Can't open the port
            println!("Can't Open Port");
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
    fn read_function_result(self: &mut Self, fun: &Function) -> Result<Vec<u8>> {
        let packed_size = |v: u16| v / 8 + if v % 8 > 0 { 1 } else { 0 };
        let (addr, count, expected_bytes) = match *fun {
            Function::ReadCoils(a, c) | Function::ReadDiscreteInputs(a, c) => {
                (a, c, packed_size(c) as usize)
            }
            Function::ReadHoldingRegisters(a, c) | Function::ReadInputRegisters(a, c) => {
                (a, c, 2 * c as usize)
            }
            _ => return Err(Error::InvalidFunction),
        };

        if count < 1 {
            return Err(Error::InvalidData(Reason::RecvBufferEmpty));
        }

        let mut msg = Vec::new();
        msg.push(self.id);
        msg.push(fun.code());

        let mut address_bytes = [0, 0];
        BigEndian::write_u16(&mut address_bytes, addr);
        msg.extend(address_bytes.iter());

        let mut count_bytes = [0, 0];
        BigEndian::write_u16(&mut count_bytes, count);
        msg.extend(count_bytes.iter());

        self.connection.write_with_crc(&msg);

        // Expected bytes is data, we also expect back the id, function code, and byte count
        let response = self
            .connection
            .read(Some(expected_bytes + 3), self.id, fun.code())?;

        let response = response[3..].to_vec();
        Ok(response)
    }

    fn write(self: &mut Self, buff: &[u8]) -> Result<()> {
        let mut writebuf = Vec::new();
        writebuf.push(self.id);
        writebuf.extend(buff.iter());
        self.connection.write_with_crc(&writebuf);
        // We expect back id (1), and 5 bytes depending on the function
        let _response = self.connection.read(Some(6), self.id, buff[0])?;
        Ok(())
    }
}
