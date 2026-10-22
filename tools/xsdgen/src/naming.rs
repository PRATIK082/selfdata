use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use xsd_parser::models::meta::MetaType;
use xsd_parser::models::{format_unknown_variant, make_type_name, Name, Naming as DefaultNaming};
use xsd_parser::traits::{NameBuilder as NameBuilderTrait, Naming as NamingTrait};
use xsd_parser::{Ident2, TypeIdent};

/// Registry that keeps track of all Rust identifiers handed out by the naming
/// layer and makes sure that every identifier is used at most once.
///
/// The XSD schema of AUTOSAR contains types that would result in the same Rust
/// identifier, e.g. the complex type `MEMORY-SECTION` and the simple type
/// `MEMORY-SECTION-TYPE` both map to `MemorySectionType`. The registry assigns
/// a unique suffix to the colliding identifiers instead of producing invalid
/// Rust code.
#[derive(Debug, Default)]
struct IdentRegistry {
    /// Maps the "logical" name of a type to the identifier that was handed out.
    assigned: HashMap<String, String>,
    /// All identifiers that are already in use.
    used: HashSet<String>,
}

impl IdentRegistry {
    fn assign(&mut self, key: String, base: String) -> String {
        if let Some(existing) = self.assigned.get(&key) {
            return existing.clone();
        }

        let mut candidate = base.clone();
        let mut counter = 2u32;

        while self.used.contains(&candidate) {
            candidate = format!("{base}{counter}");
            counter += 1;
        }

        self.used.insert(candidate.clone());
        self.assigned.insert(key, candidate.clone());

        candidate
    }
}

#[derive(Debug, Clone, Default)]
pub struct UniqueNaming {
    inner: DefaultNaming,
    registry: Arc<Mutex<IdentRegistry>>,
    variant_registry: Arc<Mutex<IdentRegistry>>,
}

impl NamingTrait for UniqueNaming {
    fn clone_boxed(&self) -> Box<dyn NamingTrait> {
        Box::new(self.clone())
    }

    fn builder(&self) -> Box<dyn NameBuilderTrait> {
        Box::new(UniqueNameBuilder::new(self.inner.builder()))
    }

    fn unify(&self, s: &str) -> String {
        self.inner.unify(s)
    }

    /// Exactly the default implementation of `xsd-parser`.
    ///
    /// The default keeps the `Named` / `Generated` distinction intact, which is
    /// required by the generator (e.g. `generate_named_types` only emits types
    /// with a named `TypeIdent`). Disambiguation of colliding Rust identifiers is
    /// done in `format_type_ident` instead, where only the final identifier text
    /// is changed.
    fn make_type_name(&self, postfixes: &[String], ty: &MetaType, ident: &TypeIdent) -> Name {
        make_type_name(self, postfixes, ty, ident)
    }

    fn make_unknown_variant(&self, id: usize) -> Ident2 {
        format_unknown_variant(id)
    }

    fn format_module_name(&self, s: &str) -> String {
        self.inner.format_module_name(s)
    }

    fn format_type_name(&self, s: &str) -> String {
        self.inner.format_type_name(s)
    }

    fn format_field_name(&self, s: &str) -> String {
        self.inner.format_field_name(s)
    }

    fn format_variant_name(&self, s: &str) -> String {
        self.inner.format_variant_name(s)
    }

    fn format_constant_name(&self, s: &str) -> String {
        self.inner.format_constant_name(s)
    }

    /// Makes sure that every Rust type identifier is used at most once.
    ///
    /// The default implementation formats the (already postfixed) name of a type
    /// and uses the result as Rust identifier. Some schemas contain types that
    /// would result in the same identifier, e.g. the complex type
    /// `MEMORY-SECTION` and the simple type `MEMORY-SECTION-TYPE` of the AUTOSAR
    /// schema both result in `MemorySectionType`.
    ///
    /// Since the identifier is created here, and the same function is used for
    /// the declaration and for all references of a type, the generated code
    /// stays consistent. Colliding identifiers get a numeric suffix appended.
    fn format_type_ident(&self, name: &Name, display_name: Option<&str>) -> Ident2 {
        let base = self.inner.format_type_name(display_name.unwrap_or(name.as_str()));

        let key = match name {
            Name::Named(_) => format!("named:{}", name.as_str()),
            Name::Generated(_) => format!("generated:{}", name.as_str()),
        };

        let key = match display_name {
            Some(display_name) => format!("{key}|{display_name}"),
            None => key,
        };

        let ident = self.registry.lock().unwrap().assign(key, base);

        quote::format_ident!("{ident}")
    }

    /// Makes sure that every Rust enum variant identifier is used at most once.
    ///
    /// Some enumerations contain values that only differ in the separator, e.g.
    /// `COM-AXIS` and `COM_AXIS`. The default implementation formats both to
    /// `ComAxis`, which results in duplicated enum variants.
    fn format_variant_ident(&self, name: &Name, display_name: Option<&str>) -> Ident2 {
        let base = self
            .inner
            .format_variant_name(display_name.unwrap_or(name.as_str()));

        let key = match name {
            Name::Named(_) => format!("named:{}", name.as_str()),
            Name::Generated(_) => format!("generated:{}", name.as_str()),
        };

        let key = match display_name {
            Some(display_name) => format!("{key}|{display_name}"),
            None => key,
        };

        let ident = self.variant_registry.lock().unwrap().assign(key, base);

        quote::format_ident!("{ident}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NameKind {
    /// Name of a named schema type.
    Type,
    /// Name of a generated content type (e.g. the content of a complex type).
    ContentType,
    /// Name of a struct field.
    Field,
}

#[derive(Debug)]
struct UniqueNameBuilder {
    inner: RefCell<Box<dyn NameBuilderTrait>>,
    kind: NameKind,
}

impl UniqueNameBuilder {
    fn new(inner: Box<dyn NameBuilderTrait>) -> Self {
        Self {
            inner: RefCell::new(inner),
            kind: NameKind::Type,
        }
    }
}

impl NameBuilderTrait for UniqueNameBuilder {
    fn finish(&self) -> Name {
        let name = self.inner.borrow().finish();

        let force_id = match self.kind {
            NameKind::Field => false,
            NameKind::Type => name.is_generated(),
            // content types are always generated, they must never share a name
            NameKind::ContentType => true,
        };

        if force_id {
            let mut inner = self.inner.borrow_mut();
            inner.set_with_id(true);
            inner.generate_unique_id();
            inner.finish()
        } else {
            name
        }
    }

    fn merge(&mut self, other: &dyn NameBuilderTrait) {
        if let Some(other) = (other as &dyn Any).downcast_ref::<Self>() {
            self.inner
                .borrow_mut()
                .merge(other.inner.borrow().as_ref());
        }
    }

    fn clone_boxed(&self) -> Box<dyn NameBuilderTrait> {
        Box::new(Self {
            inner: RefCell::new(self.inner.borrow().clone_boxed()),
            kind: self.kind,
        })
    }

    fn has_name(&self) -> bool {
        self.inner.borrow().has_name()
    }

    fn has_extension(&self) -> bool {
        self.inner.borrow().has_extension()
    }

    fn set_name(&mut self, name: String) {
        self.inner.get_mut().set_name(name);
    }

    fn set_with_id(&mut self, value: bool) {
        self.inner.get_mut().set_with_id(value);
    }

    fn set_generated(&mut self, value: bool) {
        self.inner.get_mut().set_generated(value);
    }

    fn add_extension(&mut self, replace: bool, extension: String) {
        self.inner.get_mut().add_extension(replace, extension);
    }

    fn strip_suffix(&mut self, suffix: &str) {
        self.inner.get_mut().strip_suffix(suffix);
    }

    fn generate_unique_id(&mut self) {
        self.inner.get_mut().generate_unique_id();
    }

    fn prepare_type_name(&mut self) {
        self.kind = NameKind::Type;
        self.inner.get_mut().prepare_type_name();
    }

    fn prepare_field_name(&mut self) {
        self.kind = NameKind::Field;
        self.inner.get_mut().prepare_field_name();
    }

    fn prepare_content_type_name(&mut self) {
        self.kind = NameKind::ContentType;
        self.inner.get_mut().prepare_content_type_name();
    }
}
