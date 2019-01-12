use singletonum::{Singleton, SingletonInit};
use inkwell::AddressSpace;
use inkwell::types::BasicTypeEnum;
use inkwell::attributes::Attribute;
use inkwell::context::Context;

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

#[derive(Debug, Singleton)]

pub struct LLVMAttributeFactory {
    attr_nounwind: Attribute,
    attr_nocapture: Attribute,
    attr_noalias: Attribute,
    attr_readnone: Attribute,
}

unsafe impl Sync for LLVMAttributeFactory {}
unsafe impl Send for LLVMAttributeFactory {}

impl SingletonInit for LLVMAttributeFactory {
    type Init = Context;
    fn init(context: &Context) -> Self {
        let attr_nounwind_id = Attribute::get_named_enum_kind_id("nounwind");
        let attr_nocapture_id = Attribute::get_named_enum_kind_id("nocapture");
        let attr_noalias_id = Attribute::get_named_enum_kind_id("noalias");
        let attr_readnone_id = Attribute::get_named_enum_kind_id("readnone");

        LLVMAttributeFactory {
            attr_nounwind: context.create_enum_attribute(attr_nounwind_id, 0),
            attr_nocapture: context.create_enum_attribute(attr_nocapture_id, 0),
            attr_noalias: context.create_enum_attribute(attr_noalias_id, 0),
            attr_readnone: context.create_enum_attribute(attr_readnone_id, 0)
        }
    }
}

impl LLVMAttributeFactory {
    pub fn attr_nounwind(&self) -> &Attribute {
        &self.attr_nounwind
    }

    pub fn attr_nocapture(&self) -> &Attribute {
        &self.attr_nocapture
    }

    pub fn attr_noalias(&self) -> &Attribute {
        &self.attr_noalias
    }

    pub fn attr_readnone(&self) -> &Attribute {
        &self.attr_readnone
    }
}


#[test]


fn test_llvm_attribute_factory() {
    let context = Context::create();

    let attr_factory = LLVMAttributeFactory::get_instance(&context);
    let nocapture = attr_factory.attr_nocapture();
    let nounwind = attr_factory.attr_nounwind();
    let noalias = attr_factory.attr_noalias();
    let readnone = attr_factory.attr_readnone();

    assert!(nocapture.is_enum());
    assert_eq!(nocapture.get_enum_value(), 0);
    assert_ne!(nocapture.get_enum_kind_id(), 0);

    assert!(nounwind.is_enum());
    assert_eq!(nounwind.get_enum_value(), 0);
    assert_ne!(nounwind.get_enum_kind_id(), 0);

    assert!(noalias.is_enum());
    assert_eq!(noalias.get_enum_value(), 0);
    assert_ne!(noalias.get_enum_kind_id(), 0);

    assert!(readnone.is_enum());
    assert_eq!(readnone.get_enum_value(), 0);
    assert_ne!(readnone.get_enum_kind_id(), 0);
}
