use crate::common::target::Target;
use crate::cpp_data::CppBaseSpecifier;
use crate::cpp_data::CppClassField;
use crate::cpp_data::CppEnumValue;
use crate::cpp_data::CppOriginLocation;
use crate::cpp_data::CppTypeData;
use crate::cpp_data::CppTypeDataKind;

use crate::cpp_data::CppVisibility;
use crate::cpp_function::CppFunction;

use crate::common::string_utils::JoinWithSeparator;
use crate::cpp_data::CppName;
use crate::cpp_data::CppTemplateInstantiation;
use crate::cpp_ffi_data::CppFfiItem;
use crate::cpp_type::CppType;
use crate::html_logger::escape_html;
use crate::rust_type::RustName;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppCheckerEnv {
    pub target: Target,
    pub cpp_library_version: Option<String>,
}

impl CppCheckerEnv {
    pub fn short_text(&self) -> String {
        format!(
            "{}/{:?}-{:?}-{:?}-{:?}",
            self.cpp_library_version
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("None"),
            self.target.arch,
            self.target.os,
            self.target.family,
            self.target.env
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseItemSource {
    CppParser {
        /// File name of the include file (without full path)
        include_file: String,
        /// Exact location of the declaration
        origin_location: CppOriginLocation,
    },
    ImplicitDestructor,
    TemplateInstantiation,
    NamespaceInfering,
    QtSignalArguments,
}

impl DatabaseItemSource {
    pub fn is_parser(&self) -> bool {
        match *self {
            DatabaseItemSource::CppParser { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppCheckerInfo {
    pub env: CppCheckerEnv,
    pub error: Option<String>,
}

impl CppCheckerInfo {
    pub fn error_to_log(error: &Option<String>) -> String {
        match error {
            None => "<div class='ok'>OK</div>".to_string(),
            Some(error) => format!(
                "<div class='error'>Error<br><pre>{}</pre></div>",
                escape_html(error)
            ),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppCheckerInfoList {
    pub items: Vec<CppCheckerInfo>,
}

pub enum CppCheckerAddResult {
    Added,
    Changed { old: Option<String> },
    Unchanged,
}

impl CppCheckerInfoList {
    pub fn add(&mut self, env: &CppCheckerEnv, error: Option<String>) -> CppCheckerAddResult {
        if let Some(item) = self.items.iter_mut().find(|i| &i.env == env) {
            let r = if item.error == error {
                CppCheckerAddResult::Unchanged
            } else {
                CppCheckerAddResult::Changed {
                    old: item.error.clone(),
                }
            };
            item.error = error;
            return r;
        }
        self.items.push(CppCheckerInfo {
            env: env.clone(),
            error,
        });
        CppCheckerAddResult::Added
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CppItemData {
    Namespace(CppName),
    Type(CppTypeData),
    EnumValue(CppEnumValue),
    Function(CppFunction),
    ClassField(CppClassField),
    ClassBase(CppBaseSpecifier),
    TemplateInstantiation(CppTemplateInstantiation),
    QtSignalArguments(Vec<CppType>),
}

impl CppItemData {
    pub fn is_same(&self, other: &CppItemData) -> bool {
        match (self, other) {
            (&CppItemData::Type(ref v), &CppItemData::Type(ref v2)) => v.is_same(v2),
            (&CppItemData::EnumValue(ref v), &CppItemData::EnumValue(ref v2)) => v.is_same(v2),
            (&CppItemData::Function(ref v), &CppItemData::Function(ref v2)) => v.is_same(v2),
            (&CppItemData::ClassField(ref v), &CppItemData::ClassField(ref v2)) => v.is_same(v2),
            (&CppItemData::ClassBase(ref v), &CppItemData::ClassBase(ref v2)) => v == v2,
            (
                &CppItemData::TemplateInstantiation(ref v),
                &CppItemData::TemplateInstantiation(ref v2),
            ) => v == v2,
            (&CppItemData::QtSignalArguments(ref v), &CppItemData::QtSignalArguments(ref v2)) => {
                v == v2
            }
            _ => false,
        }
    }

    pub fn all_involved_types(&self) -> Vec<CppType> {
        match *self {
            CppItemData::Type(ref t) => match t.kind {
                CppTypeDataKind::Enum => vec![CppType::Enum {
                    name: t.name.clone(),
                }],
                CppTypeDataKind::Class { ref type_base } => vec![CppType::Class(type_base.clone())],
            },
            CppItemData::EnumValue(_) | CppItemData::Namespace(_) => Vec::new(),
            CppItemData::Function(ref method) => method.all_involved_types(),
            CppItemData::ClassField(ref field) => {
                let class_type = CppType::Class(field.class_type.clone());
                vec![class_type, field.field_type.clone()]
            }
            CppItemData::ClassBase(ref base) => vec![
                CppType::Class(base.base_class_type.clone()),
                CppType::Class(base.derived_class_type.clone()),
            ],
            CppItemData::QtSignalArguments(ref args) => args.clone(),
            CppItemData::TemplateInstantiation(ref data) => data.template_arguments.clone(),
        }
    }

    pub fn as_namespace_ref(&self) -> Option<&CppName> {
        if let CppItemData::Namespace(ref data) = *self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_function_ref(&self) -> Option<&CppFunction> {
        if let CppItemData::Function(ref data) = *self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_field_ref(&self) -> Option<&CppClassField> {
        if let CppItemData::ClassField(ref data) = *self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_enum_value_ref(&self) -> Option<&CppEnumValue> {
        if let CppItemData::EnumValue(ref data) = *self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_base_ref(&self) -> Option<&CppBaseSpecifier> {
        if let CppItemData::ClassBase(ref data) = *self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_type_ref(&self) -> Option<&CppTypeData> {
        if let CppItemData::Type(ref data) = *self {
            Some(data)
        } else {
            None
        }
    }

    pub fn as_template_instantiation_ref(&self) -> Option<&CppTemplateInstantiation> {
        if let CppItemData::TemplateInstantiation(ref data) = *self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_signal_arguments_ref(&self) -> Option<&[CppType]> {
        if let CppItemData::QtSignalArguments(ref data) = *self {
            Some(data)
        } else {
            None
        }
    }

    pub fn path(&self) -> Option<String> {
        unimplemented!()
    }
}

impl Display for CppItemData {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        let s = match *self {
            CppItemData::Namespace(ref path) => format!("namespace {}", path),
            CppItemData::Type(ref type1) => match type1.kind {
                CppTypeDataKind::Enum => format!("enum {}", type1.name.to_cpp_code()),
                CppTypeDataKind::Class { ref type_base } => {
                    format!("class {}", type_base.to_cpp_pseudo_code())
                }
            },
            CppItemData::Function(ref method) => method.short_text(),
            CppItemData::EnumValue(ref value) => format!(
                "enum {} {{ {} = {}, ... }}",
                value.enum_name, value.name, value.value
            ),
            CppItemData::ClassField(ref field) => field.short_text(),
            CppItemData::ClassBase(ref class_base) => {
                let virtual_text = if class_base.is_virtual {
                    "virtual "
                } else {
                    ""
                };
                let visibility_text = match class_base.visibility {
                    CppVisibility::Public => "public",
                    CppVisibility::Protected => "protected",
                    CppVisibility::Private => "private",
                };
                let index_text = if class_base.base_index > 0 {
                    format!(" (index: {}", class_base.base_index)
                } else {
                    String::new()
                };
                format!(
                    "class {} : {}{} {}{}",
                    class_base.derived_class_type.to_cpp_pseudo_code(),
                    virtual_text,
                    visibility_text,
                    class_base.base_class_type.to_cpp_pseudo_code(),
                    index_text
                )
            }
            CppItemData::QtSignalArguments(ref args) => format!(
                "Qt signal args ({})",
                args.iter().map(|arg| arg.to_cpp_pseudo_code()).join(", ")
            ),
            CppItemData::TemplateInstantiation(ref data) => format!(
                "template instantiation: {}<{}>",
                data.class_name,
                data.template_arguments
                    .iter()
                    .map(|arg| arg.to_cpp_pseudo_code())
                    .join(", ")
            ),
        };

        f.write_str(&s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfiItem {
    pub cpp_item: CppFfiItem,
    pub checks: CppCheckerInfoList,
    pub another_rust_item: Option<()>,
}

impl FfiItem {
    pub fn new(cpp_item: CppFfiItem) -> Self {
        FfiItem {
            cpp_item,
            checks: CppCheckerInfoList::default(),
            another_rust_item: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustItem {
    pub path: RustName,
    pub naming_strategy: (),
    pub sclass_nested_path: Option<RustName>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseItem {
    pub cpp_data: CppItemData,

    pub source: DatabaseItemSource,
    pub ffi_items: Option<Vec<FfiItem>>,
    pub rust_item: Option<RustItem>,
}

/// Represents all collected data related to a crate.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub crate_name: String,
    pub items: Vec<DatabaseItem>,
    pub environments: Vec<CppCheckerEnv>,
    pub next_ffi_id: u64,
}

impl Database {
    pub fn empty(crate_name: &str) -> Database {
        Database {
            crate_name: crate_name.to_owned(),
            items: Vec::new(),
            environments: Vec::new(),
            next_ffi_id: 0,
        }
    }

    pub fn items(&self) -> &[DatabaseItem] {
        &self.items
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.environments.clear();
        self.next_ffi_id = 0;
    }

    pub fn crate_name(&self) -> &str {
        &self.crate_name
    }

    pub fn add_cpp_data(&mut self, source: DatabaseItemSource, data: CppItemData) -> bool {
        if let Some(item) = self
            .items
            .iter_mut()
            .find(|item| item.cpp_data.is_same(&data))
        {
            // parser data takes priority
            if source.is_parser() && !item.source.is_parser() {
                item.source = source;
            }
            return false;
        }
        self.items.push(DatabaseItem {
            cpp_data: data,
            source: source,
            ffi_items: None,
            rust_item: None,
        });
        true
    }

    /*
    pub fn mark_missing_cpp_data(&mut self, env: DataEnv) {
      let info = DataEnvInfo {
        is_success: false,
        ..DataEnvInfo::default()
      };
      for item in &mut self.items {
        if !item.environments.iter().any(|env2| env2.env == env) {
          item.environments.push(DataEnvWithInfo {
            env: env.clone(),
            info: info.clone(),
          });
        }
      }
    }*/
}
