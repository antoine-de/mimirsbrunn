/// for the moment we keep the old way of formating the labels, but this could change
/// the current format is '{nice name} ({city})'
/// the {nice name} being for addresses the housenumber and the street (correctly ordered)
/// and for the rest of the objects, only their names
fn format_label<'a>(
    nice_name: String,
    admins: impl Iterator<Item = &'a mimir::Admin> + Clone,
) -> String {
    let city = admins.clone().find(|adm| adm.is_city());
    let city_name = city.map(|a| a.name.to_string());

    match city_name {
        Some(n) => format!("{} ({})", nice_name, n),
        None => nice_name.to_string(),
    }
}

// Note: even if most of the format methods are the same for the moment,
// I feel it's better to split them to make them easier to update

/// format a label for a Street
pub fn format_street_label<'a>(
    name: &str,
    admins: impl Iterator<Item = &'a mimir::Admin> + Clone,
    _country_codes: &[String], // Note: for the moment the country code is not used, but this could change
) -> String {
    format_label(name.to_owned(), admins)
}

/// format a label for a Poi
pub fn format_poi_label<'a>(
    name: &str,
    admins: impl Iterator<Item = &'a mimir::Admin> + Clone,
    _country_codes: &[String],
) -> String {
    format_label(name.to_owned(), admins)
}

/// format a name and a label for an Address
pub fn format_addr_name_and_label<'a>(
    house_number: &str,
    street_name: &str,
    admins: impl Iterator<Item = &'a mimir::Admin> + Clone,
    country_codes: &[String],
) -> (String, String) {
    let place = FormatPlaceHolder::from_addr(house_number.to_owned(), street_name.to_owned());
    let nice_name = get_short_addr_label(place, admins.clone(), country_codes);

    (nice_name.clone(), format_label(nice_name, admins))
}

pub fn get_short_addr_label<'a>(
    place: FormatPlaceHolder,
    admins: impl Iterator<Item = &'a mimir::Admin> + Clone,
    country_codes: &[String],
) -> String {
    let country_code = country_codes.iter().next().map(|c| c.to_string()); // we arbitrarily take the first country code
    address_formatter::FORMATTER
        .short_addr_format_with_config(
            place.into_place(admins),
            address_formatter::Configuration {
                country_code,
                ..Default::default()
            },
        )
        .map_err(|e| warn!("impossible to format label: {}", e))
        .unwrap_or_else(|_| "".to_owned())
}

pub struct FormatPlaceHolder {
    street: String,
    // zip_code: Vec<String>, // For the moment we don't put the zip code in the label
    house_number: Option<String>,
}

impl FormatPlaceHolder {
    pub fn from_addr(house_number: String, street: String) -> Self {
        Self {
            street,
            house_number: Some(house_number),
        }
    }
    pub fn from_street(street: String) -> Self {
        Self {
            street,
            house_number: None,
        }
    }

    pub fn into_place<'b>(
        self,
        admins: impl Iterator<Item = &'b mimir::Admin>,
    ) -> address_formatter::Place {
        use address_formatter::Component;
        let mut place = address_formatter::Place::default();
        place[Component::HouseNumber] = self.house_number;
        place[Component::Road] = Some(self.street);

        for a in admins {
            if let Some(addr_equivalent) = cosmo_to_addr_formatter_type(&a.zone_type) {
                place[addr_equivalent] = Some(a.name.clone());
            }
        }
        place
    }
}

fn cosmo_to_addr_formatter_type(
    cosmo_type: &Option<cosmogony::ZoneType>,
) -> Option<address_formatter::Component> {
    use address_formatter::Component;
    match cosmo_type {
        Some(cosmogony::ZoneType::City) => Some(Component::City),
        Some(cosmogony::ZoneType::Country) => Some(Component::Country),
        Some(cosmogony::ZoneType::State) => Some(Component::State),
        Some(cosmogony::ZoneType::Suburb) => Some(Component::Suburb),
        // not sure, but it seems a cosmogony::StateDistrict is a County in address_formatter
        Some(cosmogony::ZoneType::StateDistrict) => Some(Component::County),
        _ => None,
    }
}
