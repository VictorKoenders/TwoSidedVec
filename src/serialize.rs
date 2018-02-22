
use serde::ser::{Serialize, Serializer, SerializeStruct};
use serde::de::{Deserialize, Deserializer};

impl<T: Serialize> Serialize for TwoSidedVec<T> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where
        S: Serializer {
        let mut two_sided = serializer.serialize_struct("TwoSidedVec", 2)?;
        two_sided.serialize_field("back", self.back())?;
        two_sided.serialize_field("front", self.front())?;
        two_sided.end()
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for TwoSidedVec<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        /*
         * NOTE: Although this is slightly less efficient than deserializing directly,
         * the temporary vector uses ten times less serde boilerplate.
         */
        #[derive(Deserialize)]
        struct SerializedTwoSidedVec<T> {
            back: Vec<T>,
            front: Vec<T>
        }
        let serialized = SerializedTwoSidedVec::deserialize(deserializer)?;
        let mut result = TwoSidedVec::with_capacity(serialized.back.len(), serialized.front.len());
        result.extend_front(serialized.front);
        result.extend_back(serialized.back);
        Ok(result)
    }
}