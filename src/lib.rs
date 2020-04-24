//! # yahoo! finance API
//!
//! This project provides a set of functions to receive data from the
//! the [yahoo! finance](https://finance.yahoo.com) website via their API. This project
//! is licensed under Apache 2.0 or MIT license (see files LICENSE-Apache2.0 and LICENSE-MIT).
//!
//! There is already an existing rust library [yahoo-finance-rs](https://github.com/fbriden/yahoo-finance-rs),
//! which I intended to use for my own projects. However, due some issues in the implementation (the library panics
//! in some cases if yahoo does provide somehow invalid data), I currently can't use it. Once this issue is fixed,
//! I might switch back and drop development of this library.
//!

use chrono::{DateTime,Utc};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::Value;
use std::fmt;


#[derive(Deserialize, Debug)]
pub struct YResponse {
    pub chart: YChart,
}

impl YResponse  {
    fn check_consistency(&self) -> Result<(), YahooError> {
        for stock in &self.chart.result {
            let n = stock.timestamp.len();
            if n == 0 {
                return Err(YahooError::EmptyDataSet);
            }
            let quote = &stock.indicators.quote[0];
            if quote.open.len() != n || quote.high.len() != n || quote.low.len() != n || quote.volume.len() !=n || quote.close.len() !=n {
                return Err(YahooError::DataInconsistency);
            }
            if stock.indicators.adjclose.is_some() {
                let adjclose = stock.indicators.adjclose.as_ref().unwrap();
                if adjclose[0].adjclose.len() != n {
                    return Err(YahooError::DataInconsistency);
                }
            }
        }
        Ok(())
    }

    pub fn last_quote(&self) -> Result<Quote, YahooError> {
        self.check_consistency()?;
        let stock = &self.chart.result[0];
        let n = stock.timestamp.len()-1;
        Ok(stock.indicators.get_ith_quote(stock.timestamp[n], n))
    }

    pub fn quotes(&self) -> Result<Vec<Quote>, YahooError> {
        self.check_consistency()?;
        let stock = &self.chart.result[0];
        let mut quotes = Vec::new();
        let n = stock.timestamp.len();
        for i in 0..n {
            let timestamp = stock.timestamp[i];
            quotes.push(stock.indicators.get_ith_quote(timestamp, i))
        }
        Ok(quotes)
    }
}

/// Struct for single quote
#[derive(Debug)]
pub struct Quote {
    pub timestamp: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub volume: u32,
    pub close: f64,
    pub adjclose: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct YChart {
    pub result: Vec<YQuoteBlock>,
    pub error: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct YQuoteBlock {
    pub meta: YMetaData,
    pub timestamp: Vec<u64>,
    pub indicators: QuoteBlock,
}

#[derive(Deserialize, Debug)]
pub struct YMetaData {
    pub currency: String,
    pub symbol: String,
    #[serde(rename="exchangeName")]
    pub exchange_name: String,
    #[serde(rename="instrumentType")]
    pub instrument_type: String,
    #[serde(rename="firstTradeDate")]
    pub first_trade_date: u32,
    #[serde(rename="regularMarketTime")]
    pub regular_market_time: u32,
    pub gmtoffset: i32,
    pub timezone: String,
    #[serde(rename="exchangeTimezoneName")]
    pub exchange_timezone_name: String,
    #[serde(rename="regularMarketPrice")]
    pub regular_market_price: f64,
    #[serde(rename="chartPreviousClose")]
    pub chart_previous_close: f64,
    #[serde(default)]
    #[serde(rename="previousClose")]
    pub previous_close: Option<f64>,
    #[serde(default)]
    pub scale: Option<i32>,
    #[serde(rename="priceHint")]
    pub price_hint: i32,
    #[serde(rename="currentTradingPeriod")]
    pub current_trading_period: TradingPeriod,
    #[serde(default)]
    #[serde(rename="tradingPeriods")]
    pub trading_periods: Option<Vec<Vec<PeriodInfo>>>,
    #[serde(rename="dataGranularity")]
    pub data_granularity: String,
    pub range: String,
    #[serde(rename="validRanges")]
    pub valid_ranges: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct TradingPeriod {
    pub pre: PeriodInfo,
    pub regular: PeriodInfo,
    pub post: PeriodInfo,
}

#[derive(Deserialize, Debug)]
pub struct PeriodInfo {
    pub timezone: String,
    pub start: u32,
    pub end: u32,
    pub gmtoffset: i32,
}

#[derive(Deserialize, Debug)]
pub struct QuoteBlock {
    quote: Vec<QuoteList>,
    #[serde(default)]
    adjclose: Option<Vec<AdjClose>>,
}

impl QuoteBlock {
    fn get_ith_quote(&self, timestamp: u64, i: usize) -> Quote {
        let adjclose = match &self.adjclose {
            Some(adjclose) => { Some(adjclose[0].adjclose[i]) },
            None => None,
        };
        let quote = &self.quote[0];
        Quote{
            timestamp: timestamp,
            open: quote.open[i],
            high: quote.high[i],
            low: quote.low[i],
            volume: quote.volume[i],
            close: quote.close[i],
            adjclose, 
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct AdjClose {
    adjclose: Vec<f64>,
}

#[derive(Deserialize, Debug)]
pub struct QuoteList {
    pub volume: Vec<u32>,
    pub high: Vec<f64>,
    pub close: Vec<f64>,
    pub low: Vec<f64>,
    pub open: Vec<f64>,
}

#[derive(Debug)]
pub enum YahooError {
    FetchFailed(StatusCode),
    DeserializeFailed(String),
    ConnectionFailed,
    InvalidStatusCode,
    EmptyDataSet,
    DataInconsistency,
}

impl std::error::Error for YahooError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            _ => None,
        }
    }
}

impl fmt::Display for YahooError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FetchFailed(status) => write!(f, "fetchin the data from yahoo! finance failed: with status code {}", status),
            Self::DeserializeFailed(s) => write!(f, "deserializing response from yahoo! finance failed: {}", &s),
            Self::ConnectionFailed => write!(f, "connection to yahoo! finance server failed"),
            Self::InvalidStatusCode => write!(f, "yahoo! finance return invalid status code"),
            Self::EmptyDataSet => write!(f, "yahoo! finance returned an empty data set"),
            Self::DataInconsistency => write!(f, "yahoo! finance returned inconsistent data"),
        }
    }
}

/// Container for connection parameters to yahoo! finance server
pub struct YahooConnector {
    url: &'static str,
}

impl YahooConnector {
    /// Constructor for a new instance of the yahoo  connector.
    pub fn new() -> YahooConnector {
        YahooConnector {
            url: "https://query1.finance.yahoo.com/v8/finance/chart",
        }
    }

    /// Retrieve the latest quote for the given ticker
    pub fn get_latest_quotes(&self, ticker: &str, interval: &str) -> Result<YResponse, YahooError> {
        let url: String = format!(
            "{url}/{symbol}?symbol={symbol}&interval={interval}", url=self.url, symbol=ticker, interval=interval);
        let resp = self.send_request(&url)?;
        let response: YResponse =
            serde_json::from_value(resp).map_err(|e| YahooError::DeserializeFailed(e.to_string()))?;
        Ok(response)
    }

    /// Retrieve the quote history for the given ticker form date start to end (inklusive), if available
    pub fn get_quote_history(
        &self,
        ticker: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<YResponse, YahooError> {
        let url = format!("{url}/{symbol}?symbol={symbol}&period1={start}&period2={end}&interval=1d", 
            url=self.url, symbol=ticker, start=start.timestamp(), end=end.timestamp());
        let resp = self.send_request(&url)?;
        let response: YResponse =
            serde_json::from_value(resp).map_err(|err| YahooError::DeserializeFailed(err.to_string()))?;
        Ok(response)
    }

    /// Send request to yahoo! finance server and transform response to JSON value
    fn send_request(&self, url: &str) -> Result<Value, YahooError> {
        let resp = reqwest::get(url);
        if resp.is_err() {
            return Err(YahooError::ConnectionFailed);
        }
        let mut resp = resp.unwrap();
        match resp.status() {
            StatusCode::OK => match resp.json() {
                Ok(json) => Ok(json),
                _ => Err(YahooError::InvalidStatusCode),
            },

            status => Err(YahooError::FetchFailed(status)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_get_single_quote() {
        let provider = YahooConnector::new();
        let response = provider.get_latest_quotes("AAPL", "1m").unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "AAPL");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_quote_history() {
        let provider = YahooConnector::new();
        let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
        let resp = provider.get_quote_history("AAPL", start, end).unwrap();

        assert_eq!(resp.chart.result[0].timestamp.len(), 21);
        let quotes = resp.quotes().unwrap();
        assert_eq!(quotes.len(), 21);
    }
}
