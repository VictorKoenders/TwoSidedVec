use std::fmt;
use serde::ser::{Serialize, SerializeSeq, Serializer, SerializeStruct};
use serde::de::{Deserialize, DeserializeSeed, MapAccess, SeqAccess, Deserializer, Visitor, Error};

use super::TwoSidedVec;

impl<T: Serialize> Serialize for TwoSidedVec<T> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where
        S: Serializer {
        let mut two_sided = serializer.serialize_struct("TwoSidedVec", 2)?;
        two_sided.serialize_field("back", &ReversedSlice(self.back()))?;
        two_sided.serialize_field("front", self.front())?;
        two_sided.end()
    }
}
struct ReversedSlice<'a, T: 'a>(&'a [T]);
impl<'a, T: Serialize> Serialize for ReversedSlice<'a, T> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0.iter().rev() {
            seq.serialize_element(value)?;
        }
        seq.end()
    }
}
struct TwoSidedVecVisitor<T>(TwoSidedVec<T>);
impl<'de, T: Deserialize<'de>> Visitor<'de> for TwoSidedVecVisitor<T> {
    type Value = TwoSidedVec<T>;

    #[inline]
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a TwoSidedVec")
    }
    #[inline]
    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>, {
        seq.next_element_seed(Back(&mut self.0))?
            .ok_or_else(|| A::Error::invalid_length(0, &self))?;
        seq.next_element_seed(Front(&mut self.0))?
            .ok_or_else(|| A::Error::invalid_length(1, &self))?;
        Ok(self.0)
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error> where
        A: MapAccess<'de>, {
        let mut seen_back = false;
        let mut seen_front = false;
        while let Some(key) = map.next_key()? {
            match key {
                Field::Back => {
                    if seen_back {
                        return Err(A::Error::duplicate_field("back"))
                    }
                    map.next_value_seed(Back(&mut self.0))?;
                    seen_back = true;
                },
                Field::Front => {
                    if seen_front {
                        return Err(A::Error::duplicate_field("back"))
                    }
                    map.next_value_seed(Front(&mut self.0))?;
                    seen_front = true;
                }
            }
        }
        if !seen_back {
            return Err(A::Error::missing_field("back"))
        }
        if !seen_front {
            return Err(A::Error::missing_field("front"))
        }
        Ok(self.0)
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for TwoSidedVec<T> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_struct(
            "TwoSidedVec",
            FIELDS,
            TwoSidedVecVisitor(TwoSidedVec::new())
        )
    }
}
const FIELDS: &[&str] = &["back", "front"];

pub struct Front<'a, T: 'a>(&'a mut TwoSidedVec<T>);
impl<'a, 'de, T: Deserialize<'de>> DeserializeSeed<'de> for Front<'a, T> {
    type Value = ();

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where
        D: Deserializer<'de> {
        deserializer.deserialize_seq(FrontVisitor(self.0))
    }
}

pub struct FrontVisitor<'a, T: 'a>(&'a mut TwoSidedVec<T>);
impl<'a, 'de, T: Deserialize<'de>> Visitor<'de> for FrontVisitor<'a, T> {
    type Value = ();

    #[inline]
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a TwoSidedVec front")
    }
    #[inline]
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where
        A: SeqAccess<'de>, {
        if let Some(hint) = seq.size_hint() {
            self.0.reserve_front(hint);
        }
        while let Some(element) = seq.next_element()? {
            self.0.push_front(element);
        }
        Ok(())
    }
}
pub struct Back<'a, T: 'a>(&'a mut TwoSidedVec<T>);
impl<'a, 'de, T: Deserialize<'de>> DeserializeSeed<'de> for Back<'a, T> {
    type Value = ();

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where
        D: Deserializer<'de> {
        deserializer.deserialize_seq(BackVisitor(self.0))
    }
}
pub struct BackVisitor<'a, T: 'a>(&'a mut TwoSidedVec<T>);
impl<'a, 'de, T: Deserialize<'de>> Visitor<'de> for BackVisitor<'a, T> {
    type Value = ();

    #[inline]
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a TwoSidedVec back")
    }
    #[inline]
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where
        A: SeqAccess<'de>, {
        if let Some(hint) = seq.size_hint() {
            self.0.reserve_back(hint);
        }
        while let Some(element) = seq.next_element()? {
            self.0.push_back(element);
        }
        Ok(())
    }
}

enum Field {
    Back,
    Front
}
impl<'de> Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where
        D: Deserializer<'de> {
        deserializer.deserialize_identifier(FieldVisitor)
    }
}
struct FieldVisitor;
impl<'de> Visitor<'de> for FieldVisitor {
    type Value = Field;

    #[inline]
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("`back` or `front`")
    }
    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where
        E: Error, {
        match v {
            "back" => Ok(Field::Back),
            "front" => Ok(Field::Front),
            _ => Err(E::unknown_field(v, FIELDS))
        }
    }
}
