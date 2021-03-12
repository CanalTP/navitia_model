// Copyright (C) 2017 Kisio Digital and/or its affiliates.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published by the
// Free Software Foundation, version 3.

// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>

//! The different objects contained in the navitia transit model.

#![allow(missing_docs)]

use crate::{utils::*, AddPrefix, PrefixConfiguration};
use chrono::NaiveDate;
use chrono_tz::Tz;
use derivative::Derivative;
use geo::{Geometry as GeoGeometry, Point as GeoPoint};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Div, Rem, Sub};
use std::str::FromStr;
use typed_index_collection::{impl_id, impl_with_id, Idx, WithId};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    StopArea,
    StopPoint,
    Network,
    Line,
    Route,
    #[serde(rename = "trip")]
    VehicleJourney,
    StopTime,
    LineGroup,
    Ticket,
    Company,
}

pub trait GetObjectType {
    fn get_object_type() -> ObjectType;
}

impl ObjectType {
    pub fn as_str(&self) -> &'static str {
        match *self {
            ObjectType::StopArea => "stop_area",
            ObjectType::StopPoint => "stop_point",
            ObjectType::Network => "network",
            ObjectType::Line => "line",
            ObjectType::Route => "route",
            ObjectType::VehicleJourney => "trip",
            ObjectType::StopTime => "stop_time",
            ObjectType::LineGroup => "line_group",
            ObjectType::Ticket => "ticket",
            ObjectType::Company => "company",
        }
    }
}

// We use a BTreeSet<(String,String)> because Hash{Map,Set} are memory costy.
pub type KeysValues = BTreeSet<(String, String)>;

pub trait Codes {
    fn codes(&self) -> &KeysValues;
    fn codes_mut(&mut self) -> &mut KeysValues;
}
macro_rules! impl_codes {
    ($ty:ty) => {
        impl Codes for $ty {
            fn codes(&self) -> &KeysValues {
                &self.codes
            }
            fn codes_mut(&mut self) -> &mut KeysValues {
                &mut self.codes
            }
        }
    };
}

pub trait Properties {
    fn properties(&self) -> &KeysValues;
    fn properties_mut(&mut self) -> &mut KeysValues;
}
macro_rules! impl_properties {
    ($ty:ty) => {
        impl Properties for $ty {
            fn properties(&self) -> &KeysValues {
                &self.object_properties
            }
            fn properties_mut(&mut self) -> &mut KeysValues {
                &mut self.object_properties
            }
        }
    };
}

pub type CommentLinksT = BTreeSet<String>;

impl AddPrefix for CommentLinksT {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        let updated_ids = std::mem::replace(self, BTreeSet::new());
        *self = updated_ids
            .into_iter()
            .map(|comment_id| prefix_conf.schedule_prefix(comment_id.as_str()))
            .collect();
    }
}

pub trait CommentLinks {
    fn comment_links(&self) -> &CommentLinksT;
    fn comment_links_mut(&mut self) -> &mut CommentLinksT;
}

macro_rules! impl_comment_links {
    ($ty:ty) => {
        impl CommentLinks for $ty {
            fn comment_links(&self) -> &CommentLinksT {
                &self.comment_links
            }
            fn comment_links_mut(&mut self) -> &mut CommentLinksT {
                &mut self.comment_links
            }
        }
    };
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Contributor {
    #[serde(rename = "contributor_id")]
    pub id: String,
    #[serde(rename = "contributor_name")]
    pub name: String,
    #[serde(rename = "contributor_license")]
    pub license: Option<String>,
    #[serde(rename = "contributor_website")]
    pub website: Option<String>,
}

impl AddPrefix for Contributor {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
    }
}

impl Default for Contributor {
    fn default() -> Contributor {
        Contributor {
            id: "default_contributor".to_string(),
            name: "Default contributor".to_string(),
            license: Some("Unknown license".to_string()),
            website: None,
        }
    }
}

impl_with_id!(Contributor);
impl_id!(Contributor);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum DatasetType {
    #[serde(rename = "0")]
    Theorical,
    #[serde(rename = "1")]
    Revised,
    #[serde(rename = "2")]
    Production,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ValidityPeriod {
    pub start_date: Date,
    pub end_date: Date,
}

impl Default for ValidityPeriod {
    fn default() -> ValidityPeriod {
        use chrono::{Duration, Utc};
        let duration = Duration::days(15);
        let today = Utc::today();
        let start_date = today - duration;
        let end_date = today + duration;

        ValidityPeriod {
            start_date: start_date.naive_utc(),
            end_date: end_date.naive_utc(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Dataset {
    #[serde(rename = "dataset_id")]
    pub id: String,
    pub contributor_id: String,
    #[serde(
        rename = "dataset_start_date",
        deserialize_with = "de_from_date_string",
        serialize_with = "ser_from_naive_date"
    )]
    pub start_date: Date,
    #[serde(
        rename = "dataset_end_date",
        deserialize_with = "de_from_date_string",
        serialize_with = "ser_from_naive_date"
    )]
    pub end_date: Date,
    pub dataset_type: Option<DatasetType>,
    #[serde(
        rename = "dataset_extrapolation",
        default,
        deserialize_with = "de_from_u8",
        serialize_with = "ser_from_bool"
    )]
    pub extrapolation: bool,
    #[serde(rename = "dataset_desc")]
    pub desc: Option<String>,
    #[serde(rename = "dataset_system")]
    pub system: Option<String>,
}

impl Dataset {
    pub fn new(dataset_id: String, contributor_id: String) -> Dataset {
        let validity_period = ValidityPeriod::default();

        Dataset {
            id: dataset_id,
            contributor_id,
            start_date: validity_period.start_date,
            end_date: validity_period.end_date,
            dataset_type: None,
            extrapolation: false,
            desc: None,
            system: None,
        }
    }
}

impl Default for Dataset {
    fn default() -> Dataset {
        let validity_period = ValidityPeriod::default();

        Dataset {
            id: "default_dataset".to_string(),
            contributor_id: "default_contributor".to_string(),
            start_date: validity_period.start_date,
            end_date: validity_period.end_date,
            dataset_type: None,
            extrapolation: false,
            desc: None,
            system: None,
        }
    }
}
impl_id!(Dataset);
impl_id!(Dataset, Contributor, contributor_id);
impl AddPrefix for Dataset {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
        self.contributor_id = prefix_conf.referential_prefix(self.contributor_id.as_str());
    }
}

impl WithId for Dataset {
    fn with_id(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            ..Default::default()
        }
    }
}

#[derivative(Default)]
#[derive(Derivative, Serialize, Deserialize, Debug, PartialEq)]
pub struct CommercialMode {
    #[derivative(Default(value = "\"default_commercial_mode\".into()"))]
    #[serde(rename = "commercial_mode_id")]
    pub id: String,
    #[derivative(Default(value = "\"default commercial mode\".into()"))]
    #[serde(rename = "commercial_mode_name")]
    pub name: String,
}
impl_id!(CommercialMode);

impl_with_id!(CommercialMode);

#[derive(Clone, Derivative, Serialize, Deserialize, Debug)]
#[derivative(Default)]
pub struct PhysicalMode {
    #[derivative(Default(value = "\"default_physical_mode\".into()"))]
    #[serde(rename = "physical_mode_id")]
    pub id: String,
    #[derivative(Default(value = "\"default_physical_mode\".into()"))]
    #[serde(rename = "physical_mode_name")]
    pub name: String,
    pub co2_emission: Option<f32>,
}

impl_id!(PhysicalMode);

impl Hash for PhysicalMode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.id, &self.name).hash(state);
    }
}

impl Ord for PhysicalMode {
    fn cmp(&self, other: &PhysicalMode) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for PhysicalMode {
    fn partial_cmp(&self, other: &PhysicalMode) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl PartialEq for PhysicalMode {
    fn eq(&self, other: &PhysicalMode) -> bool {
        self.id == other.id && self.name == other.name
    }
}

impl Eq for PhysicalMode {}

impl_with_id!(PhysicalMode);

#[derive(Derivative, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[derivative(Default)]
pub struct Network {
    #[derivative(Default(value = "\"default_network\".into()"))]
    #[serde(rename = "network_id")]
    pub id: String,
    #[derivative(Default(value = "\"default network\".into()"))]
    #[serde(rename = "network_name")]
    pub name: String,
    #[serde(rename = "network_url")]
    pub url: Option<String>,
    #[serde(skip)]
    pub codes: KeysValues,
    #[derivative(Default(value = "Some(chrono_tz::Europe::Paris)"))]
    #[serde(rename = "network_timezone")]
    pub timezone: Option<Tz>,
    #[serde(rename = "network_lang")]
    pub lang: Option<String>,
    #[serde(rename = "network_phone")]
    pub phone: Option<String>,
    #[serde(rename = "network_address")]
    pub address: Option<String>,
    #[serde(rename = "network_sort_order")]
    pub sort_order: Option<u32>,
}

impl_id!(Network);
impl_codes!(Network);
impl_with_id!(Network);

impl GetObjectType for Network {
    fn get_object_type() -> ObjectType {
        ObjectType::Network
    }
}

impl AddPrefix for Network {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
    }
}

#[derive(Clone, Debug, PartialEq, Ord, PartialOrd, Eq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl std::fmt::Display for Rgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let color = format!("{:02X}{:02X}{:02X}", self.red, self.green, self.blue);
        f.write_str(color.as_ref())
    }
}

#[derive(Debug)]
pub enum RgbError {
    NotHexa,
    TooLongHexa,
    TooShortHexa,
}

use std::error::Error;

impl std::fmt::Display for RgbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RgbError::NotHexa => f.write_str("RgbError_NotHexa"),
            RgbError::TooLongHexa => f.write_str("RgbError_TooLongHexa"),
            RgbError::TooShortHexa => f.write_str("RgbError_NumberOfChar"),
        }
    }
}

impl Error for RgbError {
    fn description(&self) -> &str {
        match *self {
            RgbError::NotHexa => "String is not a valid Hexadecimal value",
            RgbError::TooLongHexa => "String is too long (6 characters expected)",
            RgbError::TooShortHexa => "String is too short (6 characters expected)",
        }
    }
}

impl FromStr for Rgb {
    type Err = RgbError;

    fn from_str(color_hex: &str) -> Result<Self, Self::Err> {
        let color_dec = u32::from_str_radix(color_hex, 16).map_err(|_err| RgbError::NotHexa)?;

        if color_dec >= 1 << 24 {
            return Err(RgbError::TooLongHexa);
        }

        if color_hex.chars().count() != 6 {
            return Err(RgbError::TooShortHexa);
        }

        Ok(Rgb {
            red: (color_dec >> 16) as u8,
            green: (color_dec >> 8) as u8,
            blue: color_dec as u8,
        })
    }
}

impl ::serde::Serialize for Rgb {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> ::serde::Deserialize<'de> for Rgb {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let color_hex = String::deserialize(deserializer)?;
        Rgb::from_str(&color_hex).map_err(Error::custom)
    }
}
#[derive(Derivative, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[derivative(Default)]
pub struct Line {
    #[serde(rename = "line_id")]
    #[derivative(Default(value = "\"default_line\".into()"))]
    pub id: String,
    #[serde(rename = "line_code")]
    pub code: Option<String>,
    #[serde(skip)]
    pub codes: KeysValues,
    #[serde(skip)]
    pub object_properties: KeysValues,
    #[serde(skip)]
    pub comment_links: CommentLinksT,
    #[serde(rename = "line_name")]
    pub name: String,
    #[serde(rename = "forward_line_name")]
    pub forward_name: Option<String>,
    #[serde(skip)]
    pub forward_direction: Option<String>,
    #[serde(rename = "backward_line_name")]
    pub backward_name: Option<String>,
    #[serde(skip)]
    pub backward_direction: Option<String>,
    #[serde(
        rename = "line_color",
        default,
        deserialize_with = "de_with_invalid_option"
    )]
    pub color: Option<Rgb>,
    #[serde(
        rename = "line_text_color",
        default,
        deserialize_with = "de_with_invalid_option"
    )]
    pub text_color: Option<Rgb>,
    #[serde(rename = "line_sort_order")]
    pub sort_order: Option<u32>,
    #[derivative(Default(value = "\"default_network\".into()"))]
    pub network_id: String,
    #[derivative(Default(value = "\"default_commercial_mode\".into()"))]
    pub commercial_mode_id: String,
    pub geometry_id: Option<String>,
    #[serde(rename = "line_opening_time")]
    pub opening_time: Option<Time>,
    #[serde(rename = "line_closing_time")]
    pub closing_time: Option<Time>,
}

impl_id!(Line);
impl_id!(Line, Network, network_id);
impl_id!(Line, CommercialMode, commercial_mode_id);
impl AddPrefix for Line {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
        self.network_id = prefix_conf.referential_prefix(self.network_id.as_str());
        self.forward_direction = self
            .forward_direction
            .take()
            .map(|id| prefix_conf.referential_prefix(id.as_str()));
        self.backward_direction = self
            .backward_direction
            .take()
            .map(|id| prefix_conf.referential_prefix(id.as_str()));
        self.geometry_id = self
            .geometry_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
        self.comment_links.prefix(prefix_conf);
    }
}

impl_codes!(Line);
impl_properties!(Line);
impl_comment_links!(Line);
impl_with_id!(Line);

impl GetObjectType for Line {
    fn get_object_type() -> ObjectType {
        ObjectType::Line
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Derivative, Clone)]
#[derivative(Default)]
pub struct Route {
    #[serde(rename = "route_id")]
    #[derivative(Default(value = "\"default_route\".into()"))]
    pub id: String,
    #[serde(rename = "route_name")]
    #[derivative(Default(value = "\"default route\".into()"))]
    pub name: String,
    pub direction_type: Option<String>,
    #[serde(skip)]
    pub codes: KeysValues,
    #[serde(skip)]
    pub object_properties: KeysValues,
    #[serde(skip)]
    pub comment_links: CommentLinksT,
    #[derivative(Default(value = "\"default_line\".into()"))]
    pub line_id: String,
    pub geometry_id: Option<String>,
    pub destination_id: Option<String>,
}
impl_id!(Route);
impl_id!(Route, Line, line_id);
impl AddPrefix for Route {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
        self.line_id = prefix_conf.referential_prefix(self.line_id.as_str());

        self.geometry_id = self
            .geometry_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
        self.destination_id = self
            .destination_id
            .take()
            .map(|id| prefix_conf.referential_prefix(id.as_str()));
        self.comment_links.prefix(prefix_conf);
    }
}
impl_codes!(Route);
impl_properties!(Route);
impl_comment_links!(Route);
impl_with_id!(Route);

impl GetObjectType for Route {
    fn get_object_type() -> ObjectType {
        ObjectType::Route
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct VehicleJourney {
    #[serde(rename = "trip_id")]
    pub id: String,
    #[serde(skip)]
    pub codes: KeysValues,
    #[serde(skip)]
    pub object_properties: KeysValues,
    #[serde(skip)]
    pub comment_links: CommentLinksT,
    pub route_id: String,
    pub physical_mode_id: String,
    pub dataset_id: String,
    pub service_id: String,
    #[serde(
        default,
        rename = "trip_headsign",
        deserialize_with = "de_option_empty_string"
    )]
    pub headsign: Option<String>,
    #[serde(rename = "trip_short_name")]
    pub short_name: Option<String>,
    pub block_id: Option<String>,
    pub company_id: String,
    pub trip_property_id: Option<String>,
    pub geometry_id: Option<String>,
    #[serde(skip)]
    pub stop_times: Vec<StopTime>,
    pub journey_pattern_id: Option<String>,
}
impl Default for VehicleJourney {
    fn default() -> VehicleJourney {
        VehicleJourney {
            id: "default_vehiclejourney".to_string(),
            codes: KeysValues::default(),
            object_properties: KeysValues::default(),
            comment_links: CommentLinksT::default(),
            route_id: "default_route".to_string(),
            physical_mode_id: "default_physical_mode".to_string(),
            dataset_id: "default_dataset".to_string(),
            service_id: "default_service".to_string(),
            headsign: None,
            short_name: None,
            block_id: None,
            company_id: "default_company".to_string(),
            trip_property_id: None,
            geometry_id: None,
            stop_times: vec![],
            journey_pattern_id: None,
        }
    }
}
impl_id!(VehicleJourney);
impl_id!(VehicleJourney, Route, route_id);
impl_id!(VehicleJourney, PhysicalMode, physical_mode_id);
impl_id!(VehicleJourney, Dataset, dataset_id);
impl_id!(VehicleJourney, Company, company_id);
impl_id!(VehicleJourney, Calendar, service_id);

impl AddPrefix for VehicleJourney {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.schedule_prefix(self.id.as_str());
        self.route_id = prefix_conf.referential_prefix(self.route_id.as_str());
        self.dataset_id = prefix_conf.referential_prefix(self.dataset_id.as_str());
        self.company_id = prefix_conf.referential_prefix(self.company_id.as_str());
        self.service_id = prefix_conf.schedule_prefix(self.service_id.as_str());
        self.trip_property_id = self
            .trip_property_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
        self.geometry_id = self
            .geometry_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
        self.comment_links.prefix(prefix_conf);
    }
}
impl_codes!(VehicleJourney);
impl_properties!(VehicleJourney);
impl_comment_links!(VehicleJourney);

impl WithId for VehicleJourney {
    fn with_id(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            ..Default::default()
        }
    }
}

impl GetObjectType for VehicleJourney {
    fn get_object_type() -> ObjectType {
        ObjectType::VehicleJourney
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Frequency {
    #[serde(rename = "trip_id")]
    pub vehicle_journey_id: String,
    pub start_time: Time,
    pub end_time: Time,
    pub headway_secs: u32,
}

impl AddPrefix for Frequency {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.vehicle_journey_id = prefix_conf.schedule_prefix(self.vehicle_journey_id.as_str());
    }
}

#[derive(Debug)]
pub enum TimeError {
    WrongFormat,
    WrongValue,
}
impl From<std::num::ParseIntError> for TimeError {
    fn from(_error: std::num::ParseIntError) -> Self {
        TimeError::WrongFormat
    }
}
impl std::fmt::Display for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            TimeError::WrongFormat => f.write_str("wrong format"),
            TimeError::WrongValue => f.write_str("wrong value"),
        }
    }
}
impl Error for TimeError {
    fn description(&self) -> &str {
        match *self {
            TimeError::WrongFormat => "Time format should be HH:MM:SS",
            TimeError::WrongValue => "Minutes and Seconds should be in [0..59] range",
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Time(u32);
impl Time {
    pub fn new(h: u32, m: u32, s: u32) -> Time {
        Time(h * 60 * 60 + m * 60 + s)
    }
    pub fn hours(self) -> u32 {
        self.0 / 60 / 60
    }
    pub fn minutes(self) -> u32 {
        self.0 / 60 % 60
    }
    pub fn seconds(self) -> u32 {
        self.0 % 60
    }
    pub fn total_seconds(self) -> u32 {
        self.0
    }
}
impl Add for Time {
    type Output = Time;
    fn add(self, other: Time) -> Time {
        Time(self.total_seconds() + other.total_seconds())
    }
}
impl Sub for Time {
    type Output = Time;
    fn sub(self, other: Time) -> Time {
        Time(self.total_seconds() - other.total_seconds())
    }
}
impl Div<u32> for Time {
    type Output = Time;
    fn div(self, rhs: u32) -> Time {
        Time(self.total_seconds() / rhs)
    }
}
impl Rem<u32> for Time {
    type Output = Time;
    fn rem(self, rhs: u32) -> Time {
        Time(self.total_seconds() % rhs)
    }
}
impl FromStr for Time {
    type Err = TimeError;
    fn from_str(time: &str) -> Result<Self, Self::Err> {
        let mut t = time.split(':');
        let (hours, minutes, seconds) = match (t.next(), t.next(), t.next(), t.next()) {
            (Some(h), Some(m), Some(s), None) => (h, m, s),
            _ => return Err(TimeError::WrongFormat),
        };
        let hours: u32 = hours.parse()?;
        let minutes: u32 = minutes.parse()?;
        let last_second_number = seconds.find('.').unwrap_or_else(|| seconds.len());
        let seconds: u32 = seconds[..last_second_number].parse()?;
        if minutes > 59 || seconds > 59 {
            return Err(TimeError::WrongValue);
        }
        Ok(Time::new(hours, minutes, seconds))
    }
}

impl ::serde::Serialize for Time {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        let time = format!("{}", self);
        serializer.serialize_str(&time)
    }
}
impl<'de> ::serde::Deserialize<'de> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        use serde::de::{self, Error, Visitor};
        use std::fmt;

        // using the visitor pattern to avoid a string allocation
        struct TimeVisitor;
        impl<'de> Visitor<'de> for TimeVisitor {
            type Value = Time;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a time in the format HH:MM:SS")
            }
            fn visit_str<E: de::Error>(self, time: &str) -> Result<Time, E> {
                time.parse().map_err(Error::custom)
            }
        }

        deserializer.deserialize_str(TimeVisitor)
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02}",
            self.hours(),
            self.minutes(),
            self.seconds()
        )
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StopTime {
    pub stop_point_idx: Idx<StopPoint>,
    pub sequence: u32,
    pub arrival_time: Time,
    pub departure_time: Time,
    pub boarding_duration: u16,
    pub alighting_duration: u16,
    pub pickup_type: u8,
    pub drop_off_type: u8,
    pub datetime_estimated: bool,
    pub local_zone_id: Option<u16>,
    pub precision: Option<StopTimePrecision>,
}

impl Ord for StopTime {
    fn cmp(&self, other: &StopTime) -> Ordering {
        self.sequence.cmp(&other.sequence)
    }
}

impl PartialOrd for StopTime {
    fn partial_cmp(&self, other: &StopTime) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl GetObjectType for StopTime {
    fn get_object_type() -> ObjectType {
        ObjectType::StopTime
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum StopTimePrecision {
    #[serde(rename = "0")]
    Exact,
    #[serde(rename = "1")]
    Approximate,
    #[serde(rename = "2")]
    Estimated,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Default)]
pub struct Coord {
    pub lon: f64,
    pub lat: f64,
}

impl From<(String, String)> for Coord {
    fn from((str_lon, str_lat): (String, String)) -> Self {
        Self {
            lon: if str_lon.is_empty() {
                <f64>::default()
            } else {
                str_lon.parse::<f64>().unwrap()
            },
            lat: if str_lat.is_empty() {
                <f64>::default()
            } else {
                str_lat.parse::<f64>().unwrap()
            },
        }
    }
}

impl From<Coord> for (String, String) {
    fn from(coord: Coord) -> Self {
        (
            if (coord.lon - <f64>::default()).abs() < std::f64::EPSILON {
                "".to_string()
            } else {
                coord.lon.to_string()
            },
            if (coord.lat - <f64>::default()).abs() < std::f64::EPSILON {
                "".to_string()
            } else {
                coord.lat.to_string()
            },
        )
    }
}

// Mean Earth radius in meters
const EARTH_RADIUS: f64 = 6_371_000.0;

impl From<GeoPoint<f64>> for Coord {
    fn from(point: GeoPoint<f64>) -> Self {
        Coord {
            lon: point.x(),
            lat: point.y(),
        }
    }
}

impl Into<GeoPoint<f64>> for Coord {
    fn into(self) -> GeoPoint<f64> {
        GeoPoint::new(self.lon, self.lat)
    }
}

impl Coord {
    /// Calculate the orthodromic distance in meters
    /// between 2 geographic coordinates
    pub fn distance_to(&self, other: &Self) -> f64 {
        let phi1 = self.lat.to_radians();
        let phi2 = other.lat.to_radians();
        let lambda1 = self.lon.to_radians();
        let lambda2 = other.lon.to_radians();

        let x = f64::sin((phi2 - phi1) / 2.).powi(2);
        let y = f64::cos(phi1) * f64::cos(phi2) * f64::sin((lambda2 - lambda1) / 2.).powi(2);

        2. * EARTH_RADIUS * f64::asin(f64::sqrt(x + y))
    }

    /// Returns a proxy object allowing to compute approximate
    /// distances for cheap computation.
    ///
    /// # Example
    ///
    /// ```
    /// # use transit_model::objects::Coord;
    /// # fn get_coords() -> Vec<Coord> { vec![] }
    /// let v: Vec<Coord> = get_coords();
    /// let from = Coord { lon: 2.37715, lat: 48.846_781 };
    /// let approx = from.approx();
    /// for coord in &v {
    ///     println!("distance({:?}, {:?}) = {}", from, coord, approx.sq_distance_to(coord).sqrt());
    /// }
    /// ```
    pub fn approx(&self) -> Approx {
        let lat_rad = self.lat.to_radians();
        Approx {
            cos_lat: lat_rad.cos(),
            lon_rad: self.lon.to_radians(),
            lat_rad,
        }
    }
}

/// Proxy object to compute approximate distances.
pub struct Approx {
    cos_lat: f64,
    lon_rad: f64,
    lat_rad: f64,
}
impl Approx {
    /// Returns the squared distance to `coord`.  Squared distance is
    /// returned to skip a `sqrt` call, that is not important for
    /// distance comparison or sorting.
    ///
    /// # Example
    ///
    /// ```
    /// # use transit_model::objects::Coord;
    /// # fn get_coords() -> Vec<Coord> { vec![] }
    /// let v: Vec<Coord> = get_coords();
    /// let from = Coord { lon: 2.37715, lat: 48.846_781 };
    /// let one_km_squared = 1_000. * 1_000.;
    /// let approx = from.approx();
    /// for coord in &v {
    ///     if approx.sq_distance_to(coord) < one_km_squared {
    ///         println!("{:?} is within 1km", coord);
    ///     }
    /// }
    /// ```
    pub fn sq_distance_to(&self, coord: &Coord) -> f64 {
        fn sq(f: f64) -> f64 {
            f * f
        }
        let delta_lat = self.lat_rad - coord.lat.to_radians();
        let delta_lon = self.lon_rad - coord.lon.to_radians();
        sq(EARTH_RADIUS) * (sq(delta_lat) + sq(self.cos_lat * delta_lon))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct StopArea {
    pub id: String,
    pub name: String,
    #[serde(skip)]
    pub codes: KeysValues,
    #[serde(skip)]
    pub object_properties: KeysValues,
    #[serde(skip)]
    pub comment_links: CommentLinksT,
    pub visible: bool,
    pub coord: Coord,
    pub timezone: Option<Tz>,
    pub geometry_id: Option<String>,
    pub equipment_id: Option<String>,
    pub level_id: Option<String>,
}
impl_id!(StopArea);

impl From<StopPoint> for StopArea {
    fn from(stop_point: StopPoint) -> Self {
        StopArea {
            id: format!("Navitia:{}", stop_point.id),
            name: stop_point.name,
            codes: KeysValues::default(),
            object_properties: KeysValues::default(),
            comment_links: CommentLinksT::default(),
            visible: stop_point.visible,
            coord: stop_point.coord,
            timezone: stop_point.timezone,
            geometry_id: None,
            equipment_id: None,
            level_id: None,
        }
    }
}

impl AddPrefix for StopArea {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
        self.equipment_id = self
            .equipment_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
        self.geometry_id = self
            .geometry_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
        self.level_id = self
            .level_id
            .take()
            .map(|id| prefix_conf.referential_prefix(id.as_str()));
        self.comment_links.prefix(prefix_conf);
    }
}
impl_codes!(StopArea);
impl_properties!(StopArea);
impl_comment_links!(StopArea);
impl_with_id!(StopArea);

impl GetObjectType for StopArea {
    fn get_object_type() -> ObjectType {
        ObjectType::StopArea
    }
}
#[derivative(Default)]
#[derive(Derivative, Debug, PartialEq, Clone)]
pub enum StopType {
    #[derivative(Default)]
    Point,
    Zone,
    StopEntrance,
    GenericNode,
    BoardingArea,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct StopPoint {
    pub id: String,
    pub name: String,
    pub code: Option<String>,
    #[serde(skip)]
    pub codes: KeysValues,
    #[serde(skip)]
    pub object_properties: KeysValues,
    #[serde(skip)]
    pub comment_links: CommentLinksT,
    pub visible: bool,
    pub coord: Coord,
    pub stop_area_id: String,
    pub timezone: Option<Tz>,
    pub geometry_id: Option<String>,
    pub equipment_id: Option<String>,
    pub fare_zone_id: Option<String>,
    pub level_id: Option<String>,
    pub platform_code: Option<String>,
    #[serde(skip)]
    pub stop_type: StopType,
}

impl_id!(StopPoint);
impl_id!(StopPoint, StopArea, stop_area_id);

impl AddPrefix for StopPoint {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
        self.stop_area_id = prefix_conf.referential_prefix(self.stop_area_id.as_str());
        self.equipment_id = self
            .equipment_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
        self.geometry_id = self
            .geometry_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
        self.level_id = self
            .level_id
            .take()
            .map(|id| prefix_conf.referential_prefix(id.as_str()));
        self.comment_links.prefix(prefix_conf);
    }
}
impl_codes!(StopPoint);
impl_properties!(StopPoint);
impl_comment_links!(StopPoint);
impl_with_id!(StopPoint);

impl GetObjectType for StopPoint {
    fn get_object_type() -> ObjectType {
        ObjectType::StopPoint
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StopLocation {
    pub id: String,
    pub name: String,
    pub code: Option<String>,
    #[serde(skip)]
    pub comment_links: CommentLinksT,
    pub visible: bool,
    pub coord: Coord,
    pub parent_id: Option<String>,
    pub timezone: Option<Tz>,
    pub geometry_id: Option<String>,
    pub equipment_id: Option<String>,
    pub level_id: Option<String>,
    #[serde(skip)]
    pub stop_type: StopType,
}
impl_id!(StopLocation);
impl_comment_links!(StopLocation);

impl AddPrefix for StopLocation {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
        self.parent_id = self
            .parent_id
            .take()
            .map(|id| prefix_conf.referential_prefix(id.as_str()));
        self.geometry_id = self
            .geometry_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
        self.equipment_id = self
            .equipment_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
        self.level_id = self
            .level_id
            .take()
            .map(|id| prefix_conf.referential_prefix(id.as_str()));
        self.comment_links.prefix(prefix_conf);
    }
}

#[derivative(Default)]
#[derive(Derivative, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathwayMode {
    #[derivative(Default)]
    #[serde(rename = "1")]
    Walkway,
    #[serde(rename = "2")]
    Stairs,
    #[serde(rename = "3")]
    MovingSidewalk,
    #[serde(rename = "4")]
    Escalator,
    #[serde(rename = "5")]
    Elevator,
    #[serde(rename = "6")]
    FareGate,
    #[serde(rename = "7")]
    ExitGate,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct Pathway {
    #[serde(rename = "pathway_id")]
    pub id: String,
    pub from_stop_id: String,
    #[serde(skip)]
    pub from_stop_type: StopType,
    pub to_stop_id: String,
    #[serde(skip)]
    pub to_stop_type: StopType,
    pub pathway_mode: PathwayMode,
    #[serde(deserialize_with = "de_from_u8", serialize_with = "ser_from_bool")]
    pub is_bidirectional: bool,
    #[serde(default, deserialize_with = "de_option_positive_decimal")]
    pub length: Option<Decimal>,
    pub traversal_time: Option<u32>,
    #[serde(default, deserialize_with = "de_option_non_null_integer")]
    pub stair_count: Option<i16>,
    pub max_slope: Option<f32>,
    #[serde(default, deserialize_with = "de_option_positive_float")]
    pub min_width: Option<f32>,
    pub signposted_as: Option<String>,
    pub reversed_signposted_as: Option<String>,
}

impl AddPrefix for Pathway {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
        self.from_stop_id = prefix_conf.referential_prefix(self.from_stop_id.as_str());
        self.to_stop_id = prefix_conf.referential_prefix(self.to_stop_id.as_str());
    }
}
impl_id!(Pathway);

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct Level {
    #[serde(rename = "level_id")]
    pub id: String,
    pub level_index: f32,
    pub level_name: Option<String>,
}

impl AddPrefix for Level {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
    }
}
impl_id!(Level);

pub type Date = chrono::NaiveDate;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ExceptionType {
    #[serde(rename = "1")]
    Add,
    #[serde(rename = "2")]
    Remove,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Clone)]
pub struct Calendar {
    pub id: String,
    #[serde(skip)]
    pub dates: BTreeSet<Date>,
}

impl_id!(Calendar);
impl Calendar {
    pub fn new(calendar_id: String) -> Calendar {
        Calendar {
            id: calendar_id,
            dates: BTreeSet::new(),
        }
    }

    /// Returns true if the calendars have at least one date in common
    pub fn overlaps(&self, other: &Self) -> bool {
        !self.dates.is_disjoint(&other.dates)
    }
}

impl AddPrefix for Calendar {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.schedule_prefix(self.id.as_str());
    }
}

impl WithId for Calendar {
    fn with_id(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Company {
    #[serde(rename = "company_id")]
    pub id: String,
    #[serde(rename = "company_name")]
    pub name: String,
    #[serde(rename = "company_address")]
    pub address: Option<String>,
    #[serde(rename = "company_url")]
    pub url: Option<String>,
    #[serde(rename = "company_mail")]
    pub mail: Option<String>,
    #[serde(rename = "company_phone")]
    pub phone: Option<String>,
    #[serde(skip)]
    pub codes: KeysValues,
}

impl_id!(Company);
impl_codes!(Company);

impl Default for Company {
    fn default() -> Company {
        Company {
            id: "default_company".to_string(),
            name: "Default Company".to_string(),
            address: None,
            url: None,
            mail: None,
            phone: None,
            codes: BTreeSet::new(),
        }
    }
}
impl AddPrefix for Company {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
    }
}
impl GetObjectType for Company {
    fn get_object_type() -> ObjectType {
        ObjectType::Company
    }
}

impl_with_id!(Company);

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CommentType {
    #[derivative(Default)]
    Information,
    OnDemandTransport,
}

#[derive(Default, Serialize, Deserialize, Debug, PartialEq)]
pub struct Comment {
    #[serde(rename = "comment_id")]
    pub id: String,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub comment_type: CommentType,
    #[serde(rename = "comment_label")]
    pub label: Option<String>,
    #[serde(rename = "comment_name")]
    pub name: String,
    #[serde(rename = "comment_url")]
    pub url: Option<String>,
}

impl_id!(Comment);

impl AddPrefix for Comment {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.schedule_prefix(self.id.as_str());
    }
}

#[derive(
    Serialize, Deserialize, Debug, Derivative, PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy,
)]
#[derivative(Default)]
pub enum Availability {
    #[derivative(Default)]
    #[serde(rename = "0")]
    InformationNotAvailable,
    #[serde(rename = "1")]
    Available,
    #[serde(rename = "2")]
    NotAvailable,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Default, Clone)]
pub struct Equipment {
    #[serde(rename = "equipment_id")]
    pub id: String,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub wheelchair_boarding: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub sheltered: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub elevator: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub escalator: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub bike_accepted: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub bike_depot: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub visual_announcement: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub audible_announcement: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub appropriate_escort: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub appropriate_signage: Availability,
}

impl_id!(Equipment);

impl AddPrefix for Equipment {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.schedule_prefix(self.id.as_str());
    }
}

impl Equipment {
    pub fn is_similar(&self, other: &Self) -> bool {
        self.appropriate_escort == other.appropriate_escort
            && self.appropriate_signage == other.appropriate_signage
            && self.audible_announcement == other.audible_announcement
            && self.bike_accepted == other.bike_accepted
            && self.bike_depot == other.bike_depot
            && self.elevator == other.elevator
            && self.escalator == other.escalator
            && self.sheltered == other.sheltered
            && self.visual_announcement == other.visual_announcement
            && self.wheelchair_boarding == other.wheelchair_boarding
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transfer {
    pub from_stop_id: String,
    pub to_stop_id: String,
    #[serde(serialize_with = "ser_option_u32_with_default")]
    pub min_transfer_time: Option<u32>,
    #[serde(serialize_with = "ser_option_u32_with_default")]
    pub real_min_transfer_time: Option<u32>,
    pub equipment_id: Option<String>,
}

impl AddPrefix for Transfer {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.from_stop_id = prefix_conf.referential_prefix(self.from_stop_id.as_str());
        self.to_stop_id = prefix_conf.referential_prefix(self.to_stop_id.as_str());
        self.equipment_id = self
            .equipment_id
            .take()
            .map(|id| prefix_conf.schedule_prefix(id.as_str()));
    }
}

#[derive(Serialize, Deserialize, Debug, Derivative, PartialEq, Clone)]
#[derivative(Default)]
pub enum TransportType {
    #[derivative(Default)]
    #[serde(rename = "0")]
    Regular,
    #[serde(rename = "1")]
    ExclusiveSchool,
    #[serde(rename = "2")]
    RegularAndSchool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct TripProperty {
    #[serde(rename = "trip_property_id")]
    pub id: String,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub wheelchair_accessible: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub bike_accepted: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub air_conditioned: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub visual_announcement: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub audible_announcement: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub appropriate_escort: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub appropriate_signage: Availability,
    #[serde(deserialize_with = "de_with_empty_or_invalid_default", default)]
    pub school_vehicle_type: TransportType,
}

impl_id!(TripProperty);

impl AddPrefix for TripProperty {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.schedule_prefix(self.id.as_str());
    }
}

impl TripProperty {
    pub fn is_similar(&self, other: &Self) -> bool {
        self.air_conditioned == other.air_conditioned
            && self.appropriate_escort == other.appropriate_escort
            && self.appropriate_signage == other.appropriate_signage
            && self.audible_announcement == other.audible_announcement
            && self.bike_accepted == other.bike_accepted
            && self.school_vehicle_type == other.school_vehicle_type
            && self.visual_announcement == other.visual_announcement
            && self.wheelchair_accessible == other.wheelchair_accessible
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Geometry {
    #[serde(rename = "geometry_id")]
    pub id: String,
    #[serde(
        rename = "geometry_wkt",
        deserialize_with = "de_wkt",
        serialize_with = "ser_geometry"
    )]
    pub geometry: GeoGeometry<f64>,
}

impl_id!(Geometry);

impl AddPrefix for Geometry {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.schedule_prefix(self.id.as_str());
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct AdminStation {
    pub admin_id: String,
    pub admin_name: String,
    pub stop_id: String,
}

impl AddPrefix for AdminStation {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.admin_id = prefix_conf.referential_prefix(self.admin_id.as_str());
        self.stop_id = prefix_conf.referential_prefix(self.stop_id.as_str());
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PriceV1 {
    pub id: String,
    #[serde(
        deserialize_with = "de_from_date_string",
        serialize_with = "ser_from_naive_date"
    )]
    pub start_date: NaiveDate,
    #[serde(
        deserialize_with = "de_from_date_string",
        serialize_with = "ser_from_naive_date"
    )]
    pub end_date: NaiveDate,
    pub price: u32,
    pub name: String,
    pub ignored: String,
    pub comment: String,
    pub currency_type: Option<String>,
}

impl AddPrefix for PriceV1 {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ODFareV1 {
    #[serde(rename = "Origin ID")]
    pub origin_stop_area_id: String,
    #[serde(rename = "Origin name")]
    pub origin_name: Option<String>,
    #[serde(rename = "Origin mode")]
    pub origin_mode: String,
    #[serde(rename = "Destination ID")]
    pub destination_stop_area_id: String,
    #[serde(rename = "Destination name")]
    pub destination_name: Option<String>,
    #[serde(rename = "Destination mode")]
    pub destination_mode: String,
    pub ticket_id: String,
}

impl AddPrefix for ODFareV1 {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.ticket_id = prefix_conf.referential_prefix(self.ticket_id.as_str());
        self.origin_stop_area_id =
            prefix_conf.referential_prefix(self.origin_stop_area_id.as_str());
        self.destination_stop_area_id =
            prefix_conf.referential_prefix(self.destination_stop_area_id.as_str());
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FareV1 {
    #[serde(rename = "avant changement")]
    pub before_change: String,
    #[serde(rename = "après changement")]
    pub after_change: String,
    #[serde(rename = "début trajet")]
    pub start_trip: String,
    #[serde(rename = "fin trajet")]
    pub end_trip: String,
    #[serde(rename = "condition globale")]
    pub global_condition: String,
    #[serde(rename = "clef ticket")]
    pub ticket_id: String,
}

impl AddPrefix for FareV1 {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.ticket_id = prefix_conf.referential_prefix(self.ticket_id.as_str());
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Ticket {
    #[serde(rename = "ticket_id")]
    pub id: String,
    #[serde(rename = "ticket_name")]
    pub name: String,
    #[serde(rename = "ticket_comment")]
    pub comment: Option<String>,
}
impl_id!(Ticket);

impl GetObjectType for Ticket {
    fn get_object_type() -> ObjectType {
        ObjectType::Ticket
    }
}

impl AddPrefix for Ticket {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TicketPrice {
    pub ticket_id: String,
    #[serde(rename = "ticket_price", deserialize_with = "de_positive_decimal")]
    pub price: Decimal,
    #[serde(
        rename = "ticket_currency",
        serialize_with = "ser_currency_code",
        deserialize_with = "de_currency_code"
    )]
    pub currency: String,
    #[serde(
        deserialize_with = "de_from_date_string",
        serialize_with = "ser_from_naive_date"
    )]
    pub ticket_validity_start: Date,
    #[serde(
        deserialize_with = "de_from_date_string",
        serialize_with = "ser_from_naive_date"
    )]
    pub ticket_validity_end: Date,
}

impl AddPrefix for TicketPrice {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.ticket_id = prefix_conf.referential_prefix(self.ticket_id.as_str());
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TicketUse {
    #[serde(rename = "ticket_use_id")]
    pub id: String,
    pub ticket_id: String,
    pub max_transfers: Option<u32>,
    pub boarding_time_limit: Option<u32>,
    pub alighting_time_limit: Option<u32>,
}
impl_id!(TicketUse);

impl AddPrefix for TicketUse {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
        self.ticket_id = prefix_conf.referential_prefix(self.ticket_id.as_str());
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum PerimeterAction {
    #[serde(rename = "1")]
    Included,
    #[serde(rename = "2")]
    Excluded,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TicketUsePerimeter {
    pub ticket_use_id: String,
    pub object_type: ObjectType,
    pub object_id: String,
    pub perimeter_action: PerimeterAction,
}

impl AddPrefix for TicketUsePerimeter {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.ticket_use_id = prefix_conf.referential_prefix(self.ticket_use_id.as_str());
        self.object_id = prefix_conf.referential_prefix(self.object_id.as_str());
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum RestrictionType {
    #[serde(rename = "zone")]
    Zone,
    #[serde(rename = "OD")]
    OriginDestination,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TicketUseRestriction {
    pub ticket_use_id: String,
    pub restriction_type: RestrictionType,
    pub use_origin: String,
    pub use_destination: String,
}

impl AddPrefix for TicketUseRestriction {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.ticket_use_id = prefix_conf.referential_prefix(self.ticket_use_id.as_str());
        self.use_origin = prefix_conf.referential_prefix(self.use_origin.as_str());
        self.use_destination = prefix_conf.referential_prefix(self.use_destination.as_str());
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GridCalendar {
    #[serde(rename = "grid_calendar_id")]
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "de_from_u8", serialize_with = "ser_from_bool")]
    pub monday: bool,
    #[serde(deserialize_with = "de_from_u8", serialize_with = "ser_from_bool")]
    pub tuesday: bool,
    #[serde(deserialize_with = "de_from_u8", serialize_with = "ser_from_bool")]
    pub wednesday: bool,
    #[serde(deserialize_with = "de_from_u8", serialize_with = "ser_from_bool")]
    pub thursday: bool,
    #[serde(deserialize_with = "de_from_u8", serialize_with = "ser_from_bool")]
    pub friday: bool,
    #[serde(deserialize_with = "de_from_u8", serialize_with = "ser_from_bool")]
    pub saturday: bool,
    #[serde(deserialize_with = "de_from_u8", serialize_with = "ser_from_bool")]
    pub sunday: bool,
}
impl_id!(GridCalendar);

impl AddPrefix for GridCalendar {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.id = prefix_conf.referential_prefix(self.id.as_str());
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GridExceptionDate {
    pub grid_calendar_id: String,
    #[serde(
        deserialize_with = "de_from_date_string",
        serialize_with = "ser_from_naive_date"
    )]
    pub date: Date,
    #[serde(deserialize_with = "de_from_u8", serialize_with = "ser_from_bool")]
    pub r#type: bool,
}
impl_id!(GridExceptionDate, GridCalendar, grid_calendar_id);

impl AddPrefix for GridExceptionDate {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.grid_calendar_id = prefix_conf.referential_prefix(self.grid_calendar_id.as_str());
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GridPeriod {
    pub grid_calendar_id: String,
    #[serde(
        deserialize_with = "de_from_date_string",
        serialize_with = "ser_from_naive_date"
    )]
    pub start_date: Date,
    #[serde(
        deserialize_with = "de_from_date_string",
        serialize_with = "ser_from_naive_date"
    )]
    pub end_date: Date,
}
impl_id!(GridPeriod, GridCalendar, grid_calendar_id);

impl AddPrefix for GridPeriod {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.grid_calendar_id = prefix_conf.referential_prefix(self.grid_calendar_id.as_str());
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GridRelCalendarLine {
    pub grid_calendar_id: String,
    pub line_id: String,
    pub line_external_code: Option<String>,
}
impl_id!(GridRelCalendarLine, GridCalendar, grid_calendar_id);
impl_id!(GridRelCalendarLine, Line, line_id);

impl AddPrefix for GridRelCalendarLine {
    fn prefix(&mut self, prefix_conf: &PrefixConfiguration) {
        self.grid_calendar_id = prefix_conf.referential_prefix(self.grid_calendar_id.as_str());
        self.line_id = prefix_conf.referential_prefix(self.line_id.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use pretty_assertions::assert_eq;

    #[test]
    fn rgb_serialization() {
        let white = Rgb {
            red: 255,
            green: 255,
            blue: 255,
        };
        assert_eq!("FFFFFF", serde_json::to_value(&white).unwrap());

        let black = Rgb {
            red: 0,
            green: 0,
            blue: 0,
        };
        assert_eq!("000000", serde_json::to_value(&black).unwrap());

        let blue = Rgb {
            red: 0,
            green: 125,
            blue: 255,
        };
        assert_eq!("007DFF", serde_json::to_value(&blue).unwrap());
    }

    #[test]
    fn rgb_deserialization_with_too_big_color_hex() {
        let json_value = serde_json::Value::String("1FFFFFF".to_string());
        let rgb: Result<Rgb, _> = serde_json::from_value(json_value);

        assert!(rgb.is_err());
    }

    #[test]
    fn rgb_deserialization_with_bad_number_of_digits() {
        for &color in ["F", "FF", "FFF", "FFFF", "FFFFF"].iter() {
            let json_value = serde_json::Value::String(color.to_string());
            let rgb: Result<Rgb, _> = serde_json::from_value(json_value);

            assert!(rgb.is_err());
        }
    }

    #[test]
    fn rgb_good_deserialization() {
        let json_value = serde_json::Value::String("FFFFFF".to_string());
        let rgb: Rgb = serde_json::from_value(json_value).unwrap();

        assert_eq!(255, rgb.red);
        assert_eq!(255, rgb.green);
        assert_eq!(255, rgb.blue);

        let json_value = serde_json::Value::String("000000".to_string());
        let rgb: Rgb = serde_json::from_value(json_value).unwrap();

        assert_eq!(0, rgb.red);
        assert_eq!(0, rgb.green);
        assert_eq!(0, rgb.blue);

        let json_value = serde_json::Value::String("007DFF".to_string());
        let rgb: Rgb = serde_json::from_value(json_value).unwrap();

        assert_eq!(0, rgb.red);
        assert_eq!(125, rgb.green);
        assert_eq!(255, rgb.blue);
    }

    #[test]
    fn time_serialization() {
        let ser = |h, m, s| serde_json::to_value(&Time::new(h, m, s)).unwrap();

        assert_eq!("13:37:00", ser(13, 37, 0));
        assert_eq!("00:00:00", ser(0, 0, 0));
        assert_eq!("25:42:42", ser(25, 42, 42));
    }

    #[test]
    fn time_deserialization() {
        let de = |s: &str| serde_json::from_value(serde_json::Value::String(s.to_string()));

        assert_eq!(Time::new(13, 37, 0), de("13:37:00").unwrap());
        assert_eq!(Time::new(0, 0, 0), de("0:0:0").unwrap());
        assert_eq!(Time::new(25, 42, 42), de("25:42:42").unwrap());
        assert_eq!(Time::new(13, 37, 0), de("13:37:00.000").unwrap());

        assert!(de("").is_err());
        assert!(de("13:37").is_err());
        assert!(de("13:37:").is_err());
        assert!(de("AA:00:00").is_err());
        assert!(de("00:AA:00").is_err());
        assert!(de("00:00:AA").is_err());
    }

    // distance between COORD1 and COORD2 is 357.64 from
    // https://gps-coordinates.org/distance-between-coordinates.php
    const COORD1: Coord = Coord {
        lon: 2.377_054,
        lat: 48.846_995,
    };
    const COORD2: Coord = Coord {
        lon: 2.374_377,
        lat: 48.844_304,
    };

    const EPSILON: f64 = 0.001;

    #[test]
    fn orthodromic_distance() {
        assert_relative_eq!(COORD1.distance_to(&COORD1), 0.0);
        assert_relative_eq!(COORD1.distance_to(&COORD2), 357.644, epsilon = EPSILON);
        assert_relative_eq!(COORD2.distance_to(&COORD1), 357.644, epsilon = EPSILON);
    }

    #[test]
    fn approx_distance() {
        assert_relative_eq!(COORD1.approx().sq_distance_to(&COORD1).sqrt(), 0.0);
        assert_relative_eq!(
            COORD1.approx().sq_distance_to(&COORD2).sqrt(),
            357.642,
            epsilon = EPSILON
        );
        assert_relative_eq!(
            COORD2.approx().sq_distance_to(&COORD1).sqrt(),
            357.647,
            epsilon = EPSILON
        );
    }
}
