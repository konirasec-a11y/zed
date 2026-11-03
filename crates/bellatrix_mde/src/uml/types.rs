//! Evaluated literal values and multiplicity intervals, not ValueSpecification
//! model elements. Owned lowerValue/upperValue and TypedElement::type references
//! still require their own canonical model identities (UML 2.5.1 §§7.5, 8).

use crate::Relation;
use num_bigint::{BigInt, Sign};
use serde_json::Value;
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LiteralValueError {
    #[error("the value is not a supported decimal Integer literal")]
    UnsupportedInteger,
    #[error("a finite UnlimitedNatural cannot be negative")]
    NegativeUnlimitedNatural,
}

/// Identities from PrimitiveTypes.xmi, not user-model Type references.
/// Catalog membership alone does not implement each primitive's value semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StandardPrimitiveType {
    Boolean,
    Integer,
    Real,
    String,
    UnlimitedNatural,
}

impl StandardPrimitiveType {
    pub const NAMESPACE: &str = "http://www.omg.org/spec/PrimitiveTypes/20161101";
    pub const ALL: [Self; 5] = [
        Self::Boolean,
        Self::Integer,
        Self::Real,
        Self::String,
        Self::UnlimitedNatural,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Boolean => "Boolean",
            Self::Integer => "Integer",
            Self::Real => "Real",
            Self::String => "String",
            Self::UnlimitedNatural => "UnlimitedNatural",
        }
    }

    pub fn literal_metaclass(self) -> &'static str {
        match self {
            Self::Boolean => "LiteralBoolean",
            Self::Integer => "LiteralInteger",
            Self::Real => "LiteralReal",
            Self::String => "LiteralString",
            Self::UnlimitedNatural => "LiteralUnlimitedNatural",
        }
    }
}

// UML Integer is not limited to the host machine's integer range.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Integer(BigInt);

impl Integer {
    pub const PRIMITIVE_TYPE: StandardPrimitiveType = StandardPrimitiveType::Integer;

    pub fn is_negative(&self) -> bool {
        self.0.sign() == Sign::Minus
    }
}

impl From<u64> for Integer {
    fn from(value: u64) -> Self {
        Self(value.into())
    }
}

impl FromStr for Integer {
    type Err = LiteralValueError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(LiteralValueError::UnsupportedInteger);
        }
        text.parse()
            .map(Self)
            .map_err(|_| LiteralValueError::UnsupportedInteger)
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// None denotes the absence of a limit, not an Integer or infinity (§8.2.4).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnlimitedNatural(Option<Integer>);

impl UnlimitedNatural {
    pub const PRIMITIVE_TYPE: StandardPrimitiveType = StandardPrimitiveType::UnlimitedNatural;
    pub const UNLIMITED: Self = Self(None);

    pub fn finite_value(&self) -> Option<&Integer> {
        self.0.as_ref()
    }

    pub fn is_unlimited(&self) -> bool {
        self.0.is_none()
    }
}

impl From<u64> for UnlimitedNatural {
    fn from(value: u64) -> Self {
        Self(Some(value.into()))
    }
}

impl TryFrom<Integer> for UnlimitedNatural {
    type Error = LiteralValueError;

    fn try_from(value: Integer) -> Result<Self, Self::Error> {
        if value.is_negative() {
            Err(LiteralValueError::NegativeUnlimitedNatural)
        } else {
            Ok(Self(Some(value)))
        }
    }
}

impl FromStr for UnlimitedNatural {
    type Err = LiteralValueError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.trim() == "*" {
            Ok(Self::UNLIMITED)
        } else {
            text.parse::<Integer>()?.try_into()
        }
    }
}

impl fmt::Display for UnlimitedNatural {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.finite_value() {
            Some(value) => value.fmt(formatter),
            None => formatter.write_str("*"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MultiplicityError {
    #[error("multiplicity must be a string in the MDE representation")]
    Representation,
    #[error("constant UML ValueSpecifications are not evaluated by the literal-bound adapter")]
    UnsupportedValueSpecification,
    #[error(
        "multiplicity requires a nonnegative Integer lower bound and an UnlimitedNatural upper bound at least as large"
    )]
    InvalidBounds,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiplicityRange {
    lower: Integer,
    upper: UnlimitedNatural,
}

impl Default for MultiplicityRange {
    fn default() -> Self {
        Self {
            lower: 1.into(),
            upper: 1.into(),
        }
    }
}

impl MultiplicityRange {
    /// Missing bounds independently default to 1 (§7.8.8.7). An unevaluated
    /// authored expression is not a missing bound and must not use this fallback.
    pub fn from_bounds(
        lower: Option<Integer>,
        upper: Option<UnlimitedNatural>,
    ) -> Result<Self, MultiplicityError> {
        let lower = lower.unwrap_or_else(|| 1.into());
        let upper = upper.unwrap_or_else(|| 1.into());
        if lower.is_negative() || upper.finite_value().is_some_and(|value| value < &lower) {
            return Err(MultiplicityError::InvalidBounds);
        }
        Ok(Self { lower, upper })
    }

    pub fn lower_bound(&self) -> &Integer {
        &self.lower
    }

    pub fn upper_bound(&self) -> &UnlimitedNatural {
        &self.upper
    }

    pub fn is_multivalued(&self) -> bool {
        self.upper
            .finite_value()
            .is_none_or(|value| value > &Integer::from(1))
    }

    pub fn includes_cardinality(&self, cardinality: &Integer) -> bool {
        cardinality >= &self.lower
            && self
                .upper
                .finite_value()
                .is_none_or(|value| cardinality <= value)
    }

    pub fn includes_multiplicity(&self, other: &Self) -> bool {
        self.lower <= other.lower
            && self.upper.finite_value().is_none_or(|upper| {
                other
                    .upper
                    .finite_value()
                    .is_some_and(|other_upper| other_upper <= upper)
            })
    }

    pub fn compatible_with(&self, other: &Self) -> bool {
        other.includes_multiplicity(self)
    }
}

impl FromStr for MultiplicityRange {
    type Err = MultiplicityError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let (lower, upper) = if text == "*" {
            ("0", "*")
        } else {
            text.split_once("..").unwrap_or((text, text))
        };
        let lower = lower
            .parse()
            .map_err(|_| MultiplicityError::UnsupportedValueSpecification)?;
        let upper = upper.parse().map_err(|error| match error {
            LiteralValueError::UnsupportedInteger => {
                MultiplicityError::UnsupportedValueSpecification
            }
            LiteralValueError::NegativeUnlimitedNatural => MultiplicityError::InvalidBounds,
        })?;
        Self::from_bounds(Some(lower), Some(upper))
    }
}

/// The retained notation adapter is shared by Property and Parameter. A present
/// unevaluated bound is not absence and must never silently become 1.
pub fn literal_multiplicity(
    properties: &std::collections::BTreeMap<String, Value>,
) -> Result<MultiplicityRange, MultiplicityError> {
    if ["lowerValue", "upperValue"]
        .iter()
        .any(|key| properties.get(*key).is_some_and(|value| !value.is_null()))
    {
        return Err(MultiplicityError::UnsupportedValueSpecification);
    }
    match properties.get("multiplicity") {
        None => Ok(MultiplicityRange::default()),
        Some(Value::String(text)) => text.parse(),
        Some(_) => Err(MultiplicityError::Representation),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationEnd {
    Source,
    Target,
}

impl RelationEnd {
    pub fn multiplicity_key(self) -> &'static str {
        match self {
            Self::Source => "sourceMultiplicity",
            Self::Target => "targetMultiplicity",
        }
    }
}

/// Borrows the existing relation properties; it does not claim that the relation
/// has modeled memberEnd Properties or owned ValueSpecifications yet.
#[derive(Clone, Copy, Debug)]
pub struct RelationEndMultiplicity<'a> {
    value: Option<&'a Value>,
}

impl<'a> RelationEndMultiplicity<'a> {
    pub fn new(relation: &'a Relation, end: RelationEnd) -> Self {
        Self {
            value: relation.properties.get(end.multiplicity_key()),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_value(value: Option<&'a Value>) -> Self {
        Self { value }
    }

    pub fn authored_notation(self) -> Option<&'a str> {
        self.value.and_then(Value::as_str)
    }

    pub fn effective(self) -> Result<MultiplicityRange, MultiplicityError> {
        match self.value {
            None => Ok(MultiplicityRange::default()),
            Some(Value::String(text)) => text.parse(),
            Some(_) => Err(MultiplicityError::Representation),
        }
    }
}
