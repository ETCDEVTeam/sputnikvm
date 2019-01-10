use inkwell::AddressSpace;
use inkwell::types::BasicTypeEnum;
pub mod compiler;


pub trait BasicTypeEnumCompare {
    fn is_int_t(self) -> bool;
    fn is_int1(self) -> bool;
    fn is_int8(self) -> bool;
    fn is_int32(self) -> bool;
    fn is_int64(self) -> bool;
    fn is_int128(self) -> bool;
    fn is_int256(self) -> bool;
    fn is_ptr_type(self) -> bool;
    fn is_ptr_to_int8(self) -> bool;
    fn is_ptr_to_struct(self) -> bool;
    fn is_array_t(self) -> bool;
    fn is_array_of_len_n(self, len:u32) -> bool;
    fn is_int8_array(self, len:u32) -> bool;
}

impl BasicTypeEnumCompare for BasicTypeEnum {
    fn is_int_t(self) -> bool {
        self.is_int_type()
    }

    fn is_int1(self) -> bool {
        self.is_int_type() && (self.into_int_type().get_bit_width() == 1)
    }

    fn is_int8(self) -> bool {
        self.is_int_type() && (self.into_int_type().get_bit_width() == 8)
    }

    fn is_int32(self) -> bool {
        self.is_int_type() && (self.into_int_type().get_bit_width() == 32)
    }

    fn is_int64(self) -> bool {
        self.is_int_type() && (self.into_int_type().get_bit_width() == 64)
    }

    fn is_int128(self) -> bool {
        self.is_int_type() && (self.into_int_type().get_bit_width() == 128)
    }

    fn is_int256(self) -> bool {
        self.is_int_type() && (self.into_int_type().get_bit_width() == 256)
    }

    fn is_ptr_type(self) -> bool {
        self.is_pointer_type() &&
        (self.as_pointer_type().get_address_space() == AddressSpace::Generic)
    }

    fn is_ptr_to_int8(self) -> bool {
        if !self.is_ptr_type() {
            false;
        }

        let elem_t = self.as_pointer_type().get_element_type();
        elem_t.is_int_type() && (elem_t.as_int_type().get_bit_width() == 8)
    }

    fn is_ptr_to_struct(self) -> bool {
        if !self.is_ptr_type() {
            false;
        }

        let elem_t = self.as_pointer_type().get_element_type();
        elem_t.is_struct_type()
    }

    fn is_array_t(self) -> bool {
        self.is_array_type()
    }

    fn is_array_of_len_n(self, len : u32) -> bool {
        self.is_array_type() && (self.into_array_type().len() == len)
    }

    fn is_int8_array(self, len : u32) -> bool {
        self.is_array_of_len_n (len) &&
        self.into_array_type().get_element_type().is_int_type() &&
        (self.into_int_type().get_bit_width() == len)
    }
}

