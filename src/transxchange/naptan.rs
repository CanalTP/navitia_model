// Copyright 2017 Kisio Digital and/or its affiliates.
//
// This program is free software: you can redistribute it and/or
// modify it under the terms of the GNU General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see
// <http://www.gnu.org/licenses/>.

//! Module to help parsing and reading NaPTAN files
//! https://en.wikipedia.org/wiki/NaPTAN

use crate::{
    collection::CollectionWithId,
    model::Collections,
    objects::{Coord, StopArea, StopPoint},
    Result,
};
use failure::{format_err, ResultExt};
use geo_types::Point;
use log::info;
#[cfg(feature = "proj")]
use proj::Proj;
use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::Read, path::Path};
use zip::ZipArchive;

#[derive(Debug, Deserialize)]
pub struct NaPTANStop {
    #[serde(rename = "ATCOCode")]
    atco_code: String,
    #[serde(rename = "CommonName")]
    name: String,
    #[serde(rename = "Longitude")]
    longitude: f64,
    #[serde(rename = "Latitude")]
    latitude: f64,
    #[serde(rename = "Indicator")]
    indicator: String,
}

#[derive(Debug, Deserialize)]
pub struct NaPTANStopInArea {
    #[serde(rename = "AtcoCode")]
    atco_code: String,
    #[serde(rename = "StopAreaCode")]
    stop_area_code: String,
}

#[derive(Debug, Deserialize)]
pub struct NaPTANStopArea {
    #[serde(rename = "StopAreaCode")]
    stop_area_code: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Easting")]
    easting: f64,
    #[serde(rename = "Northing")]
    northing: f64,
}

fn read_stop_areas<R>(reader: R) -> Result<CollectionWithId<StopArea>>
where
    R: Read,
{
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b',')
        .trim(csv::Trim::All)
        .from_reader(reader);
    let mut stop_areas = CollectionWithId::default();
    let from = "EPSG:27700";
    // FIXME: String 'EPSG:4326' is failing at runtime (string below is equivalent but works)
    let to = "+proj=longlat +datum=WGS84 +no_defs"; // See https://epsg.io/4326
    let converter = Proj::new_known_crs(from, to, None)
        .ok_or_else(|| format_err!("Proj cannot build a converter from '{}' to '{}'", from, to))?;
    for record in reader.deserialize() {
        let stop_area: NaPTANStopArea =
            record.with_context(|_| "Error parsing the CSV record into a StopArea")?;
        let point = Point::new(stop_area.easting, stop_area.northing);
        let ntm_stop_area = StopArea {
            id: stop_area.stop_area_code.clone(),
            name: stop_area.name.clone(),
            coord: converter.convert(point).map(Coord::from)?,
            ..Default::default()
        };
        stop_areas.push(ntm_stop_area)?;
    }
    Ok(stop_areas)
}

fn read_stops_in_area<R>(reader: R) -> Result<HashMap<String, String>>
where
    R: Read,
{
    csv::ReaderBuilder::new()
        .delimiter(b',')
        .trim(csv::Trim::All)
        .from_reader(reader)
        .deserialize()
        .map(|record: csv::Result<NaPTANStopInArea>| {
            record.with_context(|_| "Error parsing the CSV record into a StopInArea")
        })
        .map(|record| {
            let stop_in_area = record?;
            let key_value = (
                stop_in_area.atco_code.clone(),
                stop_in_area.stop_area_code.clone(),
            );
            Ok(key_value)
        })
        .collect()
}

fn read_stops<R>(
    reader: R,
    stops_in_area: &HashMap<String, String>,
) -> Result<CollectionWithId<StopPoint>>
where
    R: Read,
{
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b',')
        .trim(csv::Trim::All)
        .from_reader(reader);
    let mut stop_points = CollectionWithId::default();
    for record in reader.deserialize() {
        let stop: NaPTANStop =
            record.with_context(|_| "Error parsing the CSV record into a Stop")?;
        let stop_area_id = stops_in_area.get(&stop.atco_code).cloned().ok_or_else(|| {
            format_err!(
                "Failed to find the corresponding StopArea for the StopPoint '{}'",
                stop.atco_code
            )
        })?;
        let coord = Coord {
            lon: stop.longitude,
            lat: stop.latitude,
        };
        let stop_point = StopPoint {
            id: stop.atco_code.clone(),
            name: stop.name.clone(),
            coord,
            stop_area_id,
            platform_code: Some(stop.indicator.clone()),
            ..Default::default()
        };
        stop_points.push(stop_point)?;
    }
    Ok(stop_points)
}

fn validate_stops(
    _stop_areas: &CollectionWithId<StopArea>,
    _stop_points: &CollectionWithId<StopPoint>,
) -> Result<()> {
    unimplemented!()
}

const STOP_AREAS_FILENAME: &str = "StopAreas.csv";
const STOPS_IN_AREA_FILENAME: &str = "StopsInArea.csv";
const STOPS_FILENAME: &str = "Stops.csv";
pub fn read_naptan<P>(naptan_path: P, collections: &mut Collections) -> Result<()>
where
    P: AsRef<Path>,
{
    let zip_file = File::open(naptan_path)?;
    let mut zip_archive = ZipArchive::new(zip_file)?;
    info!("reading NaPTAN file for {}", STOP_AREAS_FILENAME);
    let stop_areas = read_stop_areas(zip_archive.by_name(STOP_AREAS_FILENAME)?)?;
    info!("reading NaPTAN file for {}", STOPS_IN_AREA_FILENAME);
    let stops_in_area = read_stops_in_area(zip_archive.by_name(STOPS_IN_AREA_FILENAME)?)?;
    info!("reading NaPTAN file for {}", STOPS_FILENAME);
    let stop_points = read_stops(zip_archive.by_name(STOPS_FILENAME)?, &stops_in_area)?;
    validate_stops(&stop_areas, &stop_points)?;
    collections.stop_areas.try_merge(stop_areas)?;
    collections.stop_points.try_merge(stop_points)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod read_stop_areas {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parsing_works() {
            let csv_content = r#""StopAreaCode","Name","Easting","Northing"
"010G0001","Bristol Bus Station",358929,173523
"010G0002","Temple Meads",359657,172418"#;
            let stop_areas = read_stop_areas(csv_content.as_bytes()).unwrap();
            assert_eq!(stop_areas.len(), 2);
            let stop_area = stop_areas.get("010G0001").unwrap();
            assert_eq!(stop_area.name, "Bristol Bus Station");
            let stop_area = stop_areas.get("010G0002").unwrap();
            assert_eq!(stop_area.name, "Temple Meads");
        }

        #[test]
        #[should_panic]
        fn no_stop_area_code() {
            let csv_content = r#""Name","NameLang","AdministrativeAreaCode","StopAreaType","GridType","Easting","Northing"
"Temple Meads",359657,172418"#;
            read_stop_areas(csv_content.as_bytes()).unwrap();
        }

        #[test]
        #[should_panic]
        fn empty_stop_area_code() {
            let csv_content = r#""StopAreaCode","Name","NameLang","AdministrativeAreaCode","StopAreaType","GridType","Easting","Northing"
,"Bristol Bus Station",358929,173523
,"Temple Meads",359657,172418"#;
            read_stop_areas(csv_content.as_bytes()).unwrap();
        }

        #[test]
        #[should_panic]
        fn duplicate_id() {
            let csv_content = r#""StopAreaCode","Name","Easting","Northing"
"010G0001","Bristol Bus Station",358929,173523
"010G0001","Bristol Bus Station",358929,173523
"010G0002","Temple Meads",359657,172418"#;
            read_stop_areas(csv_content.as_bytes()).unwrap();
        }
    }

    mod read_stop_in_area {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parsing_works() {
            let csv_content = r#""StopAreaCode","AtcoCode"
"010G0005","01000053220"
"910GBDMNSTR","0100BDMNSTR0""#;
            let stops_in_area = read_stops_in_area(csv_content.as_bytes()).unwrap();
            assert_eq!(stops_in_area.len(), 2);
            let stop_area_code = stops_in_area.get("01000053220").unwrap();
            assert_eq!(stop_area_code, "010G0005");
            let stop_area_code = stops_in_area.get("0100BDMNSTR0").unwrap();
            assert_eq!(stop_area_code, "910GBDMNSTR");
        }

        #[test]
        #[should_panic]
        fn no_atco_code() {
            let csv_content = r#""StopAreaCode"
"010G0005"
"910GBDMNSTR""#;
            read_stops_in_area(csv_content.as_bytes()).unwrap();
        }
    }

    mod read_stops {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parsing_works() {
            let csv_content = r#""ATCOCode","CommonName","Indicator","Longitude","Latitude"
"0100053316","Broad Walk Shops","Stop B",-2.5876178397,51.4558382170
"0100053264","Alberton Road","NE-bound",-2.5407019785,51.4889912765"#;
            let mut stop_in_area = HashMap::new();
            stop_in_area.insert(String::from("0100053316"), String::from("stop-area-1"));
            stop_in_area.insert(String::from("0100053264"), String::from("stop-area-2"));
            let stop_points = read_stops(csv_content.as_bytes(), &stop_in_area).unwrap();
            assert_eq!(stop_points.len(), 2);
            let stop_point = stop_points.get("0100053316").unwrap();
            assert_eq!(stop_point.name, "Broad Walk Shops");
            let stop_point = stop_points.get("0100053264").unwrap();
            assert_eq!(stop_point.name, "Alberton Road");
        }

        #[test]
        #[should_panic(
            expected = "Failed to find the corresponding StopArea for the StopPoint \\'0100053264\\'"
        )]
        fn no_stop_area() {
            let csv_content = r#""ATCOCode","CommonName","Indicator","Longitude","Latitude"
"0100053264","Alberton Road","NE-bound",-2.5407019785,51.4889912765"#;
            let stop_in_area = HashMap::new();
            read_stops(csv_content.as_bytes(), &stop_in_area).unwrap();
        }

        #[test]
        #[should_panic]
        fn no_atco_code() {
            let csv_content = r#""CommonName","Indicator","Longitude","Latitude"
"Broad Walk Shops","Stop B",-2.5876178397,51.4558382170
"Alberton Road","NE-bound",-2.5407019785,51.4889912765"#;
            let stop_in_area = HashMap::new();
            read_stops(csv_content.as_bytes(), &stop_in_area).unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Failed to find the corresponding StopArea for the StopPoint \\'\\'"
        )]
        fn no_id() {
            let csv_content = r#""ATCOCode","CommonName","Indicator","Longitude","Latitude"
,"Broad Walk Shops","Stop B",-2.5876178397,51.4558382170
,"Alberton Road","NE-bound",-2.5407019785,51.4889912765"#;
            let mut stop_in_area = HashMap::new();
            stop_in_area.insert(String::from("0100053316"), String::from("stop-area-1"));
            stop_in_area.insert(String::from("0100053264"), String::from("stop-area-2"));
            read_stops(csv_content.as_bytes(), &stop_in_area).unwrap();
        }

        #[test]
        #[should_panic]
        fn duplicate_id() {
            let csv_content = r#""ATCOCode","CommonName","Indicator","Longitude","Latitude"
"0100053316","Broad Walk Shops","Stop B",-2.5876178397,51.4558382170
"0100053316","Broad Walk Shops","Stop B",-2.5876178397,51.4558382170
"0100053264","Alberton Road","NE-bound",-2.5407019785,51.4889912765"#;
            let mut stop_in_area = HashMap::new();
            stop_in_area.insert(String::from("0100053316"), String::from("stop-area-1"));
            stop_in_area.insert(String::from("0100053264"), String::from("stop-area-2"));
            read_stops(csv_content.as_bytes(), &stop_in_area).unwrap();
        }
    }
}