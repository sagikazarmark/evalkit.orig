use std::io::{BufRead, BufReader, Read, Write};

use serde::{Deserialize, Serialize};

use crate::schema::RUN_RESULT_SCHEMA_VERSION;
use crate::{RunMetadata, RunResult, SampleResult};

#[derive(Serialize)]
#[serde(tag = "record_type", rename_all = "snake_case")]
enum JsonlWriteRecord<'a> {
    Header {
        schema_version: &'static str,
    },
    Metadata { metadata: &'a RunMetadata },
    Sample { sample: &'a SampleResult },
}

#[derive(Deserialize)]
#[serde(tag = "record_type", rename_all = "snake_case")]
enum JsonlReadRecord {
    Header {
        schema_version: String,
    },
    Metadata { metadata: RunMetadata },
    Sample { sample: SampleResult },
}

/// Write a RunResult as JSONL.
pub fn write_jsonl(result: &RunResult, mut writer: impl Write) -> Result<(), serde_json::Error> {
    write_record(
        &mut writer,
        &JsonlWriteRecord::Header {
            schema_version: RUN_RESULT_SCHEMA_VERSION,
        },
    )?;

    write_record(
        &mut writer,
        &JsonlWriteRecord::Metadata {
            metadata: &result.metadata,
        },
    )?;

    for sample in &result.samples {
        write_record(&mut writer, &JsonlWriteRecord::Sample { sample })?;
    }

    Ok(())
}

/// Read a RunResult from JSONL.
pub fn read_jsonl(reader: impl Read) -> Result<RunResult, serde_json::Error> {
    let mut schema_version = None;
    let mut metadata = None;
    let mut samples = Vec::new();

    for line in BufReader::new(reader).lines() {
        let line = line.map_err(serde_json::Error::io)?;

        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<JsonlReadRecord>(&line)? {
            JsonlReadRecord::Header {
                schema_version: record_schema_version,
            } => {
                if schema_version.replace(record_schema_version.clone()).is_some() {
                    return Err(invalid_jsonl("JSONL contains multiple header records"));
                }

                if record_schema_version != RUN_RESULT_SCHEMA_VERSION {
                    return Err(invalid_jsonl("JSONL schema version is not supported by this reader"));
                }
            }
            JsonlReadRecord::Metadata {
                metadata: record_metadata,
            } => {
                if metadata.replace(record_metadata).is_some() {
                    return Err(invalid_jsonl("JSONL contains multiple metadata records"));
                }

                if schema_version.is_none() {
                    schema_version = Some(String::from("legacy"));
                }
            }
            JsonlReadRecord::Sample { sample } => {
                if metadata.is_none() {
                    return Err(invalid_jsonl(
                        "JSONL sample record encountered before metadata record",
                    ));
                }

                samples.push(sample);
            }
        }
    }

    Ok(RunResult {
        metadata: metadata.ok_or_else(|| invalid_jsonl("JSONL is missing a metadata record"))?,
        samples,
    })
}

fn write_record(
    writer: &mut impl Write,
    record: &JsonlWriteRecord<'_>,
) -> Result<(), serde_json::Error> {
    serde_json::to_writer(&mut *writer, record)?;
    writer.write_all(b"\n").map_err(serde_json::Error::io)
}

fn invalid_jsonl(message: &str) -> serde_json::Error {
    serde_json::Error::io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message,
    ))
}
