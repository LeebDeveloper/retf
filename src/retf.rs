#[link(name = "retf",
       vers = "0.1",
       uuid = "7ab267cc-ec76-4af2-84e6-ee4c3696d000")];
#[crate_type = "lib"];

use std::io;
use std::io::SeekCur;
use std::at_vec;
use std::str::from_chars;
use std::f64::from_str;

pub enum ErlangTerm {
    Atom(@str),
    AtomCacheRef(u8),
    Integer(i32),
    Float(f64),
    Reference(@ErlangTerm, @[u32], u8),
    Port(@ErlangTerm, u32, u8),
    Pid(@ErlangTerm, u32, u32, u8),
    Tuple(@[@ErlangTerm]),
    String(@str),
    List(@[@ErlangTerm]),
    Binary(@[u8]),
    BigInteger(u8, @[u8]),
    Fun(u8, @[u8], u32, u32, 
        @ErlangTerm, @ErlangTerm, @ErlangTerm, @ErlangTerm, @[@ErlangTerm]),
    Export(@ErlangTerm, @ErlangTerm, @ErlangTerm),
    BitBinary(u8, @[u8]),
    Nil
}

pub struct Decoder {
    reader: @io::Reader
}

pub struct Encoder {
    writer: @io::Writer
}

impl Decoder {
    fn new(reader: @io::Reader) -> Decoder {
        Decoder {
            reader : reader
        }
    }

    fn decode(&self) -> @ErlangTerm {
        let tag = self.reader.read_u8();
        match tag {
            82  => self.parse_atom_cache_ref(),
            97  => self.parse_small_integer(),
            98  => self.parse_integer(),
            99  => self.parse_float(),
            100 => self.parse_atom(false),
            101 => self.parse_reference(),
            102 => self.parse_port(),
            103 => self.parse_pid(),
            104 => self.parse_tuple(true),
            105 => self.parse_tuple(false),
            106 => @Nil, // No need to parse anything else
            107 => self.parse_string(),
            108 => self.parse_list(),
            109 => self.parse_binary(),
            110 => self.parse_biginteger(true),
            111 => self.parse_biginteger(false),
            114 => self.parse_newreference(),
            115 => self.parse_atom(true),
            117 => self.parse_fun(),
            112 => self.parse_newfun(),
            113 => self.parse_export(),
            77  => self.parse_bitbinary(),
            70  => self.parse_newfloat(),
            118 => self.parse_atom(false),
            119 => self.parse_atom(true),
            131 => self.decode(), // Just term version, go ahead
            _   => fail!("Unknown etf tag")
        }        
    }

    priv fn parse_atom_cache_ref(&self) -> @ErlangTerm {
        let v = self.reader.read_u8();
        @AtomCacheRef(v)
    }

    priv fn parse_small_integer(&self) -> @ErlangTerm {
        let v = self.reader.read_u8();
        @Integer(v as i32)
    }

    priv fn parse_integer(&self) -> @ErlangTerm {
        let v = self.reader.read_be_i32();
        @Integer(v)
    }

    priv fn parse_float(&self) -> @ErlangTerm {
        let v = self.reader.read_chars(26);
        self.reader.seek(5, SeekCur);
        let o:Option<f64> = from_str(from_chars(v));
        match o {
            Some(f) => @Float(f),
            None    => fail!("Ill-formated float")
        }
    }

    priv fn parse_atom(&self, small:bool) -> @ErlangTerm {
        let l = if small {self.reader.read_u8() as uint} 
                else {self.reader.read_be_u16() as uint};
        let v = self.reader.read_chars(l);
        let s = from_chars(v);
        @Atom(s.to_managed())
    }

    priv fn parse_reference(&self) -> @ErlangTerm {
        let n = self.decode();
        let i = self.reader.read_be_u32();
        let c = self.reader.read_u8();
        @Reference(n, @[i], c)
    }

    priv fn parse_port(&self) -> @ErlangTerm {
        let n = self.decode();
        let i = self.reader.read_be_u32();
        let c = self.reader.read_u8();
        @Port(n, i, c)
    }

    priv fn parse_pid(&self) -> @ErlangTerm {
        let n = self.decode();
        let i = self.reader.read_be_u32();
        let s = self.reader.read_be_u32();
        let c = self.reader.read_u8();
        @Pid(n, i, s, c)
    }

    priv fn _parse_vec<T>(&self, n:u32, read: &fn() -> T) -> @[T] {
        do at_vec::build |push| {
            let mut i:u32 = 0;
            while i < n {
                push(read());
                i += 1;
            }
        }
    }

    priv fn parse_tuple(&self, small:bool) -> @ErlangTerm {
        let l = if small {self.reader.read_u8() as u32} 
                else {self.reader.read_be_u32()};
        let v = self._parse_vec(l, || self.decode());
        @Tuple(v)
    }

    priv fn parse_string(&self) -> @ErlangTerm {
        let l = self.reader.read_be_u16() as uint;
        let v = self.reader.read_chars(l);
        let s = from_chars(v);
        @String(s.to_managed())
    }

    priv fn parse_list(&self) -> @ErlangTerm {
        let l = self.reader.read_be_u32();
        let v = self._parse_vec(l, || self.decode());
        let r = match self.decode() {
            @Nil => v,
            t    => at_vec::append(v, [t])
        };
        @List(r)
    }

    priv fn parse_binary(&self) -> @ErlangTerm {
        let l = self.reader.read_be_u32() as uint;
        let v = self.reader.read_bytes(l);
        @Binary(at_vec::to_managed(v))
    }

    priv fn parse_biginteger(&self, small:bool) -> @ErlangTerm {
        let l = if small {self.reader.read_u8() as u32} 
                else {self.reader.read_be_u32()};
        let s = self.reader.read_u8();
        let v = self._parse_vec(l, || self.reader.read_u8());
        @BigInteger(s, v)
    }

    priv fn parse_newreference(&self) -> @ErlangTerm {
        let l = self.reader.read_be_u16() as u32;
        if l > 3 { fail!("Too long NewReference") };
        let n = self.decode();
        let c = self.reader.read_u8();
        let v = self._parse_vec(l, || self.reader.read_be_u32());
        @Reference(n, v, c)
    }

    priv fn parse_fun(&self) -> @ErlangTerm {
        let l = self.reader.read_be_u32();
        let p = self.decode();
        let m = self.decode();
        let i = self.decode();
        let u = self.decode();
        let v = self._parse_vec(l, || self.decode());
        let t = @[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
        @Fun(0, t, 0, l, m, i, u, p, v)
    }

    priv fn parse_newfun(&self) -> @ErlangTerm {
        let _s = self.reader.read_be_u32();
        let a  = self.reader.read_u8();
        let u  = self._parse_vec(16, || self.reader.read_u8());
        let i  = self.reader.read_be_u32();
        let l  = self.reader.read_be_u32();
        let m  = self.decode();
        let oi = self.decode();
        let ou = self.decode();
        let p  = self.decode();
        let v  = self._parse_vec(l, || self.decode());
        @Fun(a, u, i, l, m, oi, ou, p, v)
    }

    priv fn parse_export(&self) -> @ErlangTerm {
        let m = self.decode();
        let f = self.decode();
        let a = self.decode();
        @Export(m, f, a)
    }

    priv fn parse_bitbinary(&self) -> @ErlangTerm {
        let l = self.reader.read_be_u32() as uint;
        let b = self.reader.read_u8();
        let v = self.reader.read_bytes(l);
        @BitBinary(b, at_vec::to_managed(v))
    }

    priv fn parse_newfloat(&self) -> @ErlangTerm {
        let f = self.reader.read_be_f64();
        @Float(f)
    }

    priv fn parse_distribution_header(&self) -> @[@ErlangTerm] {
        @[]
    }
    
}

impl Encoder {
    fn new(writer: @io::Writer) -> Encoder {
        Encoder {
            writer: writer
        }
    }

    fn encode(&self, term: &ErlangTerm) {
        
    }


}

// ---------------------
// Some testz goez here!
#[cfg(test)]
mod tests {

    use super::*;
    use std::io;

    fn test_parse(bytes: &[u8], f: &fn(&ErlangTerm)) {
        io::with_bytes_reader(bytes, |rd| {
            let decoder = Decoder::new(rd);
            f(decoder.decode())
        });
    }

    #[test]
    fn deserialize_int_test() {
        test_parse([97, 1], |i| {
            match i {
                @Integer(1) => (),
                _ => fail!()
            }});
        test_parse([98, 119, 53, 148, 0], |i| {
            match i {
                @Integer(2000000000) => (),
                _ => fail!()
            }});
    }

    #[test]
    fn deserialize_atom_test() {
        test_parse([100, 0, 2, 111, 107], |i| {
            match i {
                @Atom(string) => assert_eq!(string, @"ok"),
                _ => fail!()
            }});
        test_parse([115, 2, 111, 107], |i| {
            match i {
                @Atom(string) => assert_eq!(string, @"ok"),
                _ => fail!()
            }});
    }

    #[test]
    fn deserialize_float_test() {
        test_parse([99,49,46,50,53,48,48,48,48,48,48,48,48,
                    48,48,48,48,48,48,48,48,48,48,101,
                    45,48,49,0,0,0,0,0], |i| {
            match i {
                @Float(f) => assert_eq!(f, 1.25e-01),
                _ => fail!()
            }});
    }

    #[test]
    fn deserialize_pid_test() {
        test_parse([103,100,0,13,110,111,110,111,100,101,64,
                    110,111,104,111,115,116,0,0,0,33,0,0,0,0,0], 
            |i| {
            match i {
                @Pid(@Atom(string), 33, 0, 0) => assert_eq!(string, @"nonode@nohost"),
                _ => fail!()
            }});
    }

    #[test]
    fn deserialize_tuple_test() {
        test_parse([104,3,97,1,97,2,97,3], 
            |i| {
            match i {
                @Tuple([@Integer(1), @Integer(2), @Integer(3)]) => (),
                _ => fail!()
            }});
        test_parse([105,0,0,0,3,97,1,97,2,97,3], 
            |i| {
            match i {
                @Tuple([@Integer(1), @Integer(2), @Integer(3)]) => (),
                _ => fail!()
            }});
    }

    #[test]
    fn deserialize_string_test() {
        test_parse([107, 0, 2, 111, 107], |i| {
            match i {
                @String(string) => assert_eq!(string, @"ok"),
                _ => fail!()
            }});
    }

    #[test]
    fn deserialize_list_test() {
        test_parse([108,0,0,0,2,98,0,0,1,44,98,0,0,1,144,106], |i| {
            match i {
                @List([@Integer(300), @Integer(400)]) => (),
                _ => fail!()
            }});
        test_parse([108,0,0,0,1,98,0,0,1,44,98,0,0,1,144], |i| {
            match i {
                @List([@Integer(300), @Integer(400)]) => (),
                _ => fail!()
            }});
    }

    #[test]
    fn deserialize_binary_test() {
        test_parse([109,0,0,0,3,1,2,3], |i| {
            match i {
                @Binary([1,2,3]) => (),
                _ => fail!()
            }});
    }

    #[test]
    fn deserialize_biginteger_test() {
        test_parse([110,4,0,0,94,208,178], |i| {
            match i {
                @BigInteger(0, [0,94,208,178]) => (),
                _ => fail!()
            }});
        test_parse([110,4,1,0,94,208,178], |i| {
            match i {
                @BigInteger(1, [0,94,208,178]) => (),
                _ => fail!()
            }});
    }

    #[test]
    fn deserialize_reference_test() {
        test_parse([114,0,3,100,0,13,110,111,110,111,100,101,
                    64,110,111,104,111,115,116,0,0,0,0,188,0,
                    0,0,0,0,0,0,0], 
            |i| {
            match i {
                @Reference(@Atom(string), [188,0,0], 0) => assert_eq!(string, @"nonode@nohost"),
                _ => fail!()
            }});
    }

    #[test]
    fn deserialize_full_test() {
        test_parse([131,100,0,2,111,107], 
            |i| {
            match i {
                @Atom(string) => assert_eq!(string, @"ok"),
                _ => fail!()
            }});
    }

}