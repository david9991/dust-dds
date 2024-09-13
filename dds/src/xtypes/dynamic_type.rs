use super::{error::XTypesError, type_object::TypeKind};

pub use dust_dds_derive::XTypesDynamicType;

pub type ObjectName = &'static str;

pub enum ExtensibilityKind {
    Final,
    Appendable,
    Mutable,
}
pub enum TryConstructKind {
    UseDefault,
    Discard,
    Trim,
}

pub struct TypeDescriptor {
    pub kind: TypeKind,
    pub name: ObjectName,
    // pub DynamicType base_type;
    // pub DynamicType discriminator_type;
    // pub bound: BoundSeq
    // @optional public DynamicType element_type;
    // @optional public DynamicType key_element_type;
    pub extensibility_kind: ExtensibilityKind,
    pub is_nested: bool,
}

pub type MemberId = u32;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct MemberDescriptor {
    pub name: ObjectName,
    pub id: MemberId,
    // pub _type: &dyn DynamicType,
    pub default_value: &'static str,
    pub index: u32,
    // pub label :UnionCaseLabelSeq,
    // pub try_construct_kind: TryConstructKind,
    pub is_key: bool,
    pub is_optional: bool,
    pub is_must_understand: bool,
    pub is_shared: bool,
    pub is_default_label: bool,
}

pub trait DynamicType {
    fn get_descriptor(&self) -> Result<TypeDescriptor, XTypesError>;
    fn get_name(&self) -> ObjectName;
    fn get_kind(&self) -> TypeKind;

    // DDS::ReturnCode_t get_member_by_name(inout DynamicTypeMember member, in ObjectName name);
    // DDS::ReturnCode_t get_all_members_by_name(inout DynamicTypeMembersByName member);
    // DDS::ReturnCode_t get_member(inout DynamicTypeMember member, in MemberId id);
    // DDS::ReturnCode_t get_all_members(inout DynamicTypeMembersById member);
    fn get_member_count(&self) -> u32;
    fn get_member_by_index(&self, index: u32) -> Result<impl DynamicTypeMember, XTypesError>;
    // fn get_annotation_count(&self) -> u32;
    // DDS::ReturnCode_t get_annotation(inout AnnotationDescriptor descriptor, in unsigned long idx);
    // unsigned long get_verbatim_text_count();
    // DDS::ReturnCode_t get_verbatim_text(inout VerbatimTextDescriptor descriptor, in unsigned long idx);
}

pub trait DynamicTypeMember {
    fn get_descriptor(&self) -> Result<MemberDescriptor, XTypesError>;
    // unsigned long get_annotation_count();
    // DDS::ReturnCode_t get_annotation(inout AnnotationDescriptor descriptor, in unsigned long idx);
    // unsigned long get_verbatim_text_count();
    // DDS::ReturnCode_t get_verbatim_text(inout VerbatimTextDescriptor descriptor, in unsigned long idx);

    fn get_id(&self) -> MemberId;
    fn get_name(&self) -> ObjectName;
}

impl DynamicTypeMember for MemberDescriptor {
    fn get_descriptor(&self) -> Result<MemberDescriptor, XTypesError> {
        Ok(*self)
    }

    fn get_id(&self) -> MemberId {
        self.id
    }

    fn get_name(&self) -> ObjectName {
        self.name
    }
}
