use crate::errors; 

use std::path::PathBuf;
use std::env;

use shuttle_secrets::SecretStore;
use google_sheets4::api::ValueRange;
use google_sheets4::{self, Sheets};
use serde_json::Value;

// Represents a row in the excel sheet
#[derive(Copy, Clone)]
pub struct Row<'a>{
    pub serial_number: usize,
    pub name: &'a str,
    pub roll_number: &'a str,
    pub seat_number: u32,
    pub time_in: &'a str,
    pub time_out: &'a str,
}

impl<'a> Row<'a> {
    // FEATURE: Convert to Discord Embed
    pub fn pretty_print(&self) -> String {
        let message = format!("Appended data:\nSerial Number: {}\tName: {}\tRoll Number: {}\t
                              \t\tSeat Number: {}\tTime In: {}\t Time Out: {}\t", 
                              self.serial_number, self.name, self.roll_number, self.seat_number, self.time_in, self.time_out);
        return message;
    }
}

impl<'a> From<Row<'a>> for ValueRange {
    fn from(value: Row) -> Self {

        let values = Some(vec![vec![
                          Value::String(value.serial_number.to_string()),
                          Value::String(value.name.to_owned()),
                          Value::String(value.roll_number.to_owned()),
                          Value::String(value.seat_number.to_string()),
                          Value::String(value.time_in.to_owned()),
                          Value::String(value.time_out.to_owned())
        ]]);
        let range = format!("'{}'!1:6", chrono::Local::now().with_timezone(&chrono_tz::Asia::Kolkata).format("%e %b"));

        ValueRange { 
            major_dimension: Some(String::from("ROWS")), 
            range: Some(range),
            values 
        }
    }
}

// Central object to maintan state and access Google Sheets API
pub type SheetsHub = Sheets<hyper_rustls::HttpsConnector<yup_oauth2::hyper::client::HttpConnector>>;

// Builds SheetsHub from SERVICE_ACCOUNT_CREDENTIALS through HTTPConnector
pub async fn build_hub(secret_store: &SecretStore) -> Result<SheetsHub, errors::BuildHubError> {
    // !WARNING: Do not expose sa_credentials
    let sa_credentials_path = secret_store.get("SA_CREDENTIALS_PATH").expect("SA_CREDENTIALS_PATH must be set");
    // Log sucess
    // Get path to service_account_credentials.json
    let mut path = PathBuf::new();
    path.push(env::current_dir()?);
    path.push(sa_credentials_path);

    // Auth using SA CREDENTIALS
    let sa_credentials = yup_oauth2::read_service_account_key(path)
        .await?;
    // Log sucess
    let auth = yup_oauth2::ServiceAccountAuthenticator::builder(sa_credentials)
        .build()
        .await?;
    // Log sucess

    // Build google_sheets client through HttpConnector
    let hyper_client_builder = &google_sheets4::hyper::Client::builder();
    let http_connector_builder = hyper_rustls::HttpsConnectorBuilder::new();
    let http_connector_builder_options = http_connector_builder
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();
    // Log sucess

    Ok(Sheets::new(hyper_client_builder.build(http_connector_builder_options), auth))
}

// Buggy, does not increment after 6
// This function should return the serial number to be added in the latest append
pub async fn get_next_empty_row(secret_store: &SecretStore, range: &str, spreadsheet_id: &str) -> Option<usize> {
    // CAUTION: Should handle this error safely
    let hub = build_hub(secret_store).await.unwrap();
    // Log
    let response = hub.spreadsheets().values_get(spreadsheet_id, range).doit().await.unwrap();
    // Log
    let values = response.1;
    if let Some(rows) = values.values {
        return Some(rows.len());
    }
    None
}

pub async fn append_values_to_sheet(spreadsheet_id: &str, hub: SheetsHub, value_range: ValueRange) -> Result<(), ()>{
    let range = value_range.range.clone().unwrap();
    let result = hub.spreadsheets().values_append(value_range, spreadsheet_id, range.as_str())
        .value_input_option("USER_ENTERED")
        .doit()
        .await;
    // Log
    match result {
        Ok(_) => return Ok(()),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            return Err(());
        }
    }
    // Log
}
