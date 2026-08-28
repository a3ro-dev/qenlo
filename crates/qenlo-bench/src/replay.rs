//! Validate a completed exact-CPU oracle export before reusing its truth.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use qenlo_bench::{MetadataDistribution, OracleFilter, dataset::Dataset};

use super::{Result, validate_results};

pub struct Truth {
    pub tuning: Vec<Vec<u64>>,
    pub evaluation: Vec<Vec<u64>>,
}

fn properties(path: &Path) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        let (key, value) = line.split_once('=').ok_or("invalid reference property")?;
        if values.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err("duplicate reference property".into());
        }
    }
    Ok(values)
}

pub fn load(
    path: &Path,
    data: &Dataset,
    distribution: MetadataDistribution,
    filter: OracleFilter,
    eligible: usize,
) -> Result<Truth> {
    let config = properties(&path.join("configuration.txt"))?;
    let summary = properties(&path.join("summary.txt"))?;
    for (key, expected) in [
        ("status", "completed"),
        ("recall_target_passed", "true"),
        ("filter_violations", "0"),
    ] {
        if summary.get(key).map(String::as_str) != Some(expected) {
            return Err(format!(
                "oracle reference requires completed exact CPU correctness: {key}"
            )
            .into());
        }
    }
    let target: f64 = config
        .get("recall_target")
        .ok_or("oracle reference has no recall target")?
        .parse()?;
    for key in ["tuning_recall_at_10", "evaluation_recall_at_10"] {
        let recall: f64 = summary
            .get(key)
            .ok_or_else(|| format!("oracle reference has no {key}"))?
            .parse()?;
        if !recall.is_finite() || recall + 1e-12 < target {
            return Err(format!("oracle reference recall below target: {key}").into());
        }
    }
    let spec = data.spec;
    for (key, expected) in [
        ("backend", "cpu".into()),
        ("replay_format", "qenlo-csv-v1".into()),
        ("dataset_crc32", format!("{:08x}", data.checksum)),
        ("dimensions", spec.dimension.to_string()),
        ("rows", spec.corpus.to_string()),
        ("seed", spec.seed.to_string()),
        ("k", "10".into()),
        ("corpus_range", format!("0..{}", spec.corpus)),
        (
            "tuning_range",
            format!("{}..{}", spec.corpus, spec.corpus + spec.tuning),
        ),
        (
            "evaluation_range",
            format!(
                "{}..{}",
                spec.corpus + spec.tuning,
                spec.corpus + spec.tuning + spec.evaluation
            ),
        ),
        ("metadata", distribution.label().into()),
        ("eligible_count", eligible.to_string()),
        (
            "filter_user_id",
            filter.user_id.map(|v| v.to_string()).unwrap_or_default(),
        ),
        (
            "filter_timestamp_from",
            filter
                .timestamp_from
                .map(|v| v.to_string())
                .unwrap_or_default(),
        ),
        (
            "filter_timestamp_to",
            filter
                .timestamp_to
                .map(|v| v.to_string())
                .unwrap_or_default(),
        ),
    ] {
        if config.get(key) != Some(&expected) {
            return Err(format!("oracle reference differs from current workload: {key}").into());
        }
    }
    let mut metadata = BufReader::new(File::open(path.join("metadata.csv"))?).lines();
    if metadata.next().transpose()?.as_deref() != Some("id,user_id,timestamp_micros") {
        return Err("invalid oracle reference metadata header".into());
    }
    for row in &data.corpus {
        let expected = format!("{},{},{}", row.id, row.user_id, row.timestamp_micros);
        if metadata.next().transpose()?.as_deref() != Some(expected.as_str()) {
            return Err("oracle reference metadata differs from generated corpus".into());
        }
    }
    if metadata.next().is_some() {
        return Err("trailing oracle reference metadata".into());
    }
    let mut tuning = vec![None; spec.tuning];
    let mut evaluation = vec![None; spec.evaluation];
    let mut truth = BufReader::new(File::open(path.join("truth.csv"))?).lines();
    if truth.next().transpose()?.as_deref() != Some("split,query_index,ids") {
        return Err("invalid oracle reference truth header".into());
    }
    for row in truth {
        let row = row?;
        let fields: Vec<_> = row.split(',').collect();
        if fields.len() != 3 {
            return Err("invalid oracle reference truth row".into());
        }
        let split = match fields[0] {
            "tuning" => &mut tuning,
            "evaluation" => &mut evaluation,
            _ => return Err("unknown oracle reference query split".into()),
        };
        let index: usize = fields[1].parse()?;
        let slot = split
            .get_mut(index)
            .ok_or("oracle reference query index out of range")?;
        if slot.is_some() {
            return Err("duplicate oracle reference query index".into());
        }
        let ids = if fields[2].is_empty() {
            Vec::new()
        } else {
            fields[2]
                .split(';')
                .map(str::parse)
                .collect::<std::result::Result<Vec<u64>, _>>()?
        };
        if ids.len() != eligible.min(10) {
            return Err("oracle reference truth cardinality differs from min(k, eligible)".into());
        }
        validate_results(&data.corpus, filter, &ids)?;
        *slot = Some(ids);
    }
    Ok(Truth {
        tuning: tuning
            .into_iter()
            .collect::<Option<_>>()
            .ok_or("missing oracle reference tuning query")?,
        evaluation: evaluation
            .into_iter()
            .collect::<Option<_>>()
            .ok_or("missing oracle reference evaluation query")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qenlo_bench::{
        dataset::{self, DatasetSpec},
        exact_cosine_search,
    };

    #[test]
    fn reference_requires_exact_matching_complete_and_valid_exports() {
        let path = std::env::temp_dir().join(format!("qenlo-replay-{}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        let spec = DatasetSpec {
            dimension: 2,
            corpus: 12,
            tuning: 1,
            evaluation: 1,
            seed: 42,
        };
        dataset::prepare(&path.join("data.qnb"), spec, None).unwrap();
        let mut data = dataset::load(&path.join("data.qnb"), 2, 4096).unwrap();
        super::super::metadata(&mut data.corpus, MetadataDistribution::Independent, 42);
        let filter = OracleFilter::default();
        let config = format!(
            "backend=cpu\nreplay_format=qenlo-csv-v1\ndataset_crc32={:08x}\ndimensions=2\nrows=12\nseed=42\nk=10\ncorpus_range=0..12\ntuning_range=12..13\nevaluation_range=13..14\nmetadata=synthetic-independent\neligible_count=12\nfilter_user_id=\nfilter_timestamp_from=\nfilter_timestamp_to=\nrecall_target=0.95\n",
            data.checksum
        );
        std::fs::write(path.join("configuration.txt"), &config).unwrap();
        std::fs::write(path.join("summary.txt"), "status=completed\ntuning_recall_at_10=1\nevaluation_recall_at_10=1\nrecall_target_passed=true\nfilter_violations=0\n").unwrap();
        let metadata = "id,user_id,timestamp_micros\n".to_owned()
            + &data
                .corpus
                .iter()
                .map(|row| format!("{},{},{}\n", row.id, row.user_id, row.timestamp_micros))
                .collect::<String>();
        std::fs::write(path.join("metadata.csv"), &metadata).unwrap();
        let mut truth = "split,query_index,ids\n".to_owned();
        for (split, query) in [
            ("tuning", &data.tuning[0]),
            ("evaluation", &data.evaluation[0]),
        ] {
            let ids = exact_cosine_search(&data.corpus, query, filter, 10).unwrap();
            truth += &format!(
                "{split},0,{}\n",
                ids.iter()
                    .map(|hit| hit.id.to_string())
                    .collect::<Vec<_>>()
                    .join(";")
            );
        }
        std::fs::write(path.join("truth.csv"), &truth).unwrap();
        assert!(load(&path, &data, MetadataDistribution::Independent, filter, 12).is_ok());
        let output = path.join("replayed");
        super::super::run_cli(vec![
            "run".into(),
            "--dataset".into(),
            path.join("data.qnb").display().to_string(),
            "--output".into(),
            output.display().to_string(),
            "--dimensions".into(),
            "2".into(),
            "--fraction".into(),
            "1".into(),
            "--oracle-reference".into(),
            path.display().to_string(),
            "--warmups".into(),
            "1".into(),
            "--repetitions".into(),
            "1".into(),
        ])
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(output.join("truth.csv")).unwrap(),
            truth
        );
        assert!(
            std::fs::read_to_string(output.join("configuration.txt"))
                .unwrap()
                .contains("oracle_reference_truth_crc32=")
        );
        for entry in std::fs::read_dir(&output).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }
        std::fs::remove_dir(output).unwrap();
        for damaged in [
            config.replace("backend=cpu", "backend=usearch"),
            config.replace("rows=12", "rows=13"),
            config.replace("filter_user_id=\n", "filter_user_id=0\n"),
            config.replace("dataset_crc32=", "wrong_crc32="),
        ] {
            std::fs::write(path.join("configuration.txt"), damaged).unwrap();
            assert!(load(&path, &data, MetadataDistribution::Independent, filter, 12).is_err());
        }
        std::fs::write(path.join("configuration.txt"), config).unwrap();
        for damaged in [
            "split,query_index,ids\n".to_owned(),
            truth.clone() + "tuning,0,0\n",
            truth.replace("evaluation,0,", "evaluation,1,"),
            "split,query_index,ids\ntuning,0,0;0;0;0;0;0;0;0;0;0\n".into(),
        ] {
            std::fs::write(path.join("truth.csv"), damaged).unwrap();
            assert!(load(&path, &data, MetadataDistribution::Independent, filter, 12).is_err());
        }
        std::fs::write(path.join("truth.csv"), truth).unwrap();
        std::fs::write(path.join("metadata.csv"), metadata + "12,0,0\n").unwrap();
        assert!(load(&path, &data, MetadataDistribution::Independent, filter, 12).is_err());
        for name in [
            "data.qnb",
            "configuration.txt",
            "summary.txt",
            "metadata.csv",
            "truth.csv",
        ] {
            std::fs::remove_file(path.join(name)).unwrap();
        }
        std::fs::remove_dir(path).unwrap();
    }
}
