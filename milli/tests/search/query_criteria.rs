use std::cmp::Reverse;
use std::io::Cursor;

use big_s::S;
use heed::EnvOpenOptions;
use itertools::Itertools;
use maplit::hashset;
use milli::documents::{DocumentsBatchBuilder, DocumentsBatchReader};
use milli::update::{IndexDocuments, IndexDocumentsConfig, IndexerConfig, Settings};
use milli::{AscDesc, Criterion, Index, Member, Search, SearchResult};
use rand::Rng;
use Criterion::*;

use crate::search::{self, EXTERNAL_DOCUMENTS_IDS};

const ALLOW_TYPOS: bool = true;
const DISALLOW_TYPOS: bool = false;
const ALLOW_OPTIONAL_WORDS: bool = true;
const DISALLOW_OPTIONAL_WORDS: bool = false;
const ASC_DESC_CANDIDATES_THRESHOLD: usize = 1000;

macro_rules! test_criterion {
    ($func:ident, $optional_word:ident, $authorize_typos:ident, $criteria:expr, $sort_criteria:expr) => {
        #[test]
        fn $func() {
            let criteria = $criteria;
            let index = search::setup_search_index_with_criteria(&criteria);
            let rtxn = index.read_txn().unwrap();

            let mut search = Search::new(&rtxn, &index);
            search.query(search::TEST_QUERY);
            search.limit(EXTERNAL_DOCUMENTS_IDS.len());
            search.authorize_typos($authorize_typos);
            search.optional_words($optional_word);
            search.sort_criteria($sort_criteria);

            let SearchResult { documents_ids, .. } = search.execute().unwrap();

            let expected_external_ids: Vec<_> = search::expected_order(
                &criteria,
                $authorize_typos,
                $optional_word,
                &$sort_criteria[..],
            )
            .into_iter()
            .map(|d| d.id)
            .collect();
            let documents_ids = search::internal_to_external_ids(&index, &documents_ids);
            assert_eq!(documents_ids, expected_external_ids);
        }
    };
}

test_criterion!(none_allow_typo, ALLOW_OPTIONAL_WORDS, ALLOW_TYPOS, vec![], vec![]);
test_criterion!(none_disallow_typo, DISALLOW_OPTIONAL_WORDS, DISALLOW_TYPOS, vec![], vec![]);
test_criterion!(words_allow_typo, ALLOW_OPTIONAL_WORDS, ALLOW_TYPOS, vec![Words], vec![]);
test_criterion!(
    attribute_allow_typo,
    DISALLOW_OPTIONAL_WORDS,
    ALLOW_TYPOS,
    vec![Attribute],
    vec![]
);
test_criterion!(typo, DISALLOW_OPTIONAL_WORDS, ALLOW_TYPOS, vec![Typo], vec![]);
test_criterion!(
    attribute_disallow_typo,
    DISALLOW_OPTIONAL_WORDS,
    DISALLOW_TYPOS,
    vec![Attribute],
    vec![]
);
test_criterion!(
    exactness_allow_typo,
    DISALLOW_OPTIONAL_WORDS,
    ALLOW_TYPOS,
    vec![Exactness],
    vec![]
);
test_criterion!(
    exactness_disallow_typo,
    DISALLOW_OPTIONAL_WORDS,
    DISALLOW_TYPOS,
    vec![Exactness],
    vec![]
);
test_criterion!(
    proximity_allow_typo,
    DISALLOW_OPTIONAL_WORDS,
    ALLOW_TYPOS,
    vec![Proximity],
    vec![]
);
test_criterion!(
    proximity_disallow_typo,
    DISALLOW_OPTIONAL_WORDS,
    DISALLOW_TYPOS,
    vec![Proximity],
    vec![]
);
test_criterion!(
    asc_allow_typo,
    DISALLOW_OPTIONAL_WORDS,
    ALLOW_TYPOS,
    vec![Asc(S("asc_desc_rank"))],
    vec![]
);
test_criterion!(
    asc_disallow_typo,
    DISALLOW_OPTIONAL_WORDS,
    DISALLOW_TYPOS,
    vec![Asc(S("asc_desc_rank"))],
    vec![]
);
test_criterion!(
    desc_allow_typo,
    DISALLOW_OPTIONAL_WORDS,
    ALLOW_TYPOS,
    vec![Desc(S("asc_desc_rank"))],
    vec![]
);
test_criterion!(
    desc_disallow_typo,
    DISALLOW_OPTIONAL_WORDS,
    DISALLOW_TYPOS,
    vec![Desc(S("asc_desc_rank"))],
    vec![]
);
test_criterion!(
    asc_unexisting_field_allow_typo,
    DISALLOW_OPTIONAL_WORDS,
    ALLOW_TYPOS,
    vec![Asc(S("unexisting_field"))],
    vec![]
);
test_criterion!(
    asc_unexisting_field_disallow_typo,
    DISALLOW_OPTIONAL_WORDS,
    DISALLOW_TYPOS,
    vec![Asc(S("unexisting_field"))],
    vec![]
);
test_criterion!(
    desc_unexisting_field_allow_typo,
    DISALLOW_OPTIONAL_WORDS,
    ALLOW_TYPOS,
    vec![Desc(S("unexisting_field"))],
    vec![]
);
test_criterion!(
    desc_unexisting_field_disallow_typo,
    DISALLOW_OPTIONAL_WORDS,
    DISALLOW_TYPOS,
    vec![Desc(S("unexisting_field"))],
    vec![]
);
test_criterion!(empty_sort_by_allow_typo, DISALLOW_OPTIONAL_WORDS, ALLOW_TYPOS, vec![Sort], vec![]);
test_criterion!(
    empty_sort_by_disallow_typo,
    DISALLOW_OPTIONAL_WORDS,
    DISALLOW_TYPOS,
    vec![Sort],
    vec![]
);
test_criterion!(
    sort_by_asc_allow_typo,
    DISALLOW_OPTIONAL_WORDS,
    ALLOW_TYPOS,
    vec![Sort],
    vec![AscDesc::Asc(Member::Field(S("tag")))]
);
test_criterion!(
    sort_by_asc_disallow_typo,
    DISALLOW_OPTIONAL_WORDS,
    DISALLOW_TYPOS,
    vec![Sort],
    vec![AscDesc::Asc(Member::Field(S("tag")))]
);
test_criterion!(
    sort_by_desc_allow_typo,
    DISALLOW_OPTIONAL_WORDS,
    ALLOW_TYPOS,
    vec![Sort],
    vec![AscDesc::Desc(Member::Field(S("tag")))]
);
test_criterion!(
    sort_by_desc_disallow_typo,
    DISALLOW_OPTIONAL_WORDS,
    DISALLOW_TYPOS,
    vec![Sort],
    vec![AscDesc::Desc(Member::Field(S("tag")))]
);
test_criterion!(
    default_criteria_order,
    ALLOW_OPTIONAL_WORDS,
    ALLOW_TYPOS,
    vec![Words, Typo, Proximity, Attribute, Exactness],
    vec![]
);

#[test]
fn criteria_mixup() {
    use Criterion::*;
    let index = search::setup_search_index_with_criteria(&vec![
        Words,
        Attribute,
        Desc(S("asc_desc_rank")),
        Exactness,
        Proximity,
        Typo,
    ]);

    #[rustfmt::skip]
    let criteria_mix = {
        // Criterion doesn't implement Copy, we create a new Criterion using a closure
        let desc = || Desc(S("asc_desc_rank"));
        // all possible criteria order
        vec![
            vec![Words, Attribute,  desc(),     Exactness,  Proximity,  Typo],
            vec![Words, Attribute,  desc(),     Exactness,  Typo,       Proximity],
            vec![Words, Attribute,  desc(),     Proximity,  Exactness,  Typo],
            vec![Words, Attribute,  desc(),     Proximity,  Typo,       Exactness],
            vec![Words, Attribute,  desc(),     Typo,       Exactness,  Proximity],
            vec![Words, Attribute,  desc(),     Typo,       Proximity,  Exactness],
            vec![Words, Attribute,  Exactness,  desc(),     Proximity,  Typo],
            vec![Words, Attribute,  Exactness,  desc(),     Typo,       Proximity],
            vec![Words, Attribute,  Exactness,  Proximity,  desc(),     Typo],
            vec![Words, Attribute,  Exactness,  Proximity,  Typo,       desc()],
            vec![Words, Attribute,  Exactness,  Typo,       desc(),     Proximity],
            vec![Words, Attribute,  Exactness,  Typo,       Proximity,  desc()],
            vec![Words, Attribute,  Proximity,  desc(),     Exactness,  Typo],
            vec![Words, Attribute,  Proximity,  desc(),     Typo,       Exactness],
            vec![Words, Attribute,  Proximity,  Exactness,  desc(),     Typo],
            vec![Words, Attribute,  Proximity,  Exactness,  Typo,       desc()],
            vec![Words, Attribute,  Proximity,  Typo,       desc(),     Exactness],
            vec![Words, Attribute,  Proximity,  Typo,       Exactness,  desc()],
            vec![Words, Attribute,  Typo,       desc(),     Exactness,  Proximity],
            vec![Words, Attribute,  Typo,       desc(),     Proximity,  Exactness],
            vec![Words, Attribute,  Typo,       Exactness,  desc(),     Proximity],
            vec![Words, Attribute,  Typo,       Exactness,  Proximity,  desc()],
            vec![Words, Attribute,  Typo,       Proximity,  desc(),     Exactness],
            vec![Words, Attribute,  Typo,       Proximity,  Exactness,  desc()],
            vec![Words, desc(),     Attribute,  Exactness,  Proximity,  Typo],
            vec![Words, desc(),     Attribute,  Exactness,  Typo,       Proximity],
            vec![Words, desc(),     Attribute,  Proximity,  Exactness,  Typo],
            vec![Words, desc(),     Attribute,  Proximity,  Typo,       Exactness],
            vec![Words, desc(),     Attribute,  Typo,       Exactness,  Proximity],
            vec![Words, desc(),     Attribute,  Typo,       Proximity,  Exactness],
            vec![Words, desc(),     Exactness,  Attribute,  Proximity,  Typo],
            vec![Words, desc(),     Exactness,  Attribute,  Typo,       Proximity],
            vec![Words, desc(),     Exactness,  Proximity,  Attribute,  Typo],
            vec![Words, desc(),     Exactness,  Proximity,  Typo,       Attribute],
            vec![Words, desc(),     Exactness,  Typo,       Attribute,  Proximity],
            vec![Words, desc(),     Exactness,  Typo,       Proximity,  Attribute],
            vec![Words, desc(),     Proximity,  Attribute,  Exactness,  Typo],
            vec![Words, desc(),     Proximity,  Attribute,  Typo,       Exactness],
            vec![Words, desc(),     Proximity,  Exactness,  Attribute,  Typo],
            vec![Words, desc(),     Proximity,  Exactness,  Typo,       Attribute],
            vec![Words, desc(),     Proximity,  Typo,       Attribute,  Exactness],
            vec![Words, desc(),     Proximity,  Typo,       Exactness,  Attribute],
            vec![Words, desc(),     Typo,       Attribute,  Exactness,  Proximity],
            vec![Words, desc(),     Typo,       Attribute,  Proximity,  Exactness],
            vec![Words, desc(),     Typo,       Exactness,  Attribute,  Proximity],
            vec![Words, desc(),     Typo,       Exactness,  Proximity,  Attribute],
            vec![Words, desc(),     Typo,       Proximity,  Attribute,  Exactness],
            vec![Words, desc(),     Typo,       Proximity,  Exactness,  Attribute],
            vec![Words, Exactness,  Attribute,  desc(),     Proximity,  Typo],
            vec![Words, Exactness,  Attribute,  desc(),     Typo,       Proximity],
            vec![Words, Exactness,  Attribute,  Proximity,  desc(),     Typo],
            vec![Words, Exactness,  Attribute,  Proximity,  Typo,       desc()],
            vec![Words, Exactness,  Attribute,  Typo,       desc(),     Proximity],
            vec![Words, Exactness,  Attribute,  Typo,       Proximity,  desc()],
            vec![Words, Exactness,  desc(),     Attribute,  Proximity,  Typo],
            vec![Words, Exactness,  desc(),     Attribute,  Typo,       Proximity],
            vec![Words, Exactness,  desc(),     Proximity,  Attribute,  Typo],
            vec![Words, Exactness,  desc(),     Proximity,  Typo,       Attribute],
            vec![Words, Exactness,  desc(),     Typo,       Attribute,  Proximity],
            vec![Words, Exactness,  desc(),     Typo,       Proximity,  Attribute],
            vec![Words, Exactness,  Proximity,  Attribute,  desc(),     Typo],
            vec![Words, Exactness,  Proximity,  Attribute,  Typo,       desc()],
            vec![Words, Exactness,  Proximity,  desc(),     Attribute,  Typo],
            vec![Words, Exactness,  Proximity,  desc(),     Typo,       Attribute],
            vec![Words, Exactness,  Proximity,  Typo,       Attribute,  desc()],
            vec![Words, Exactness,  Proximity,  Typo,       desc(),     Attribute],
            vec![Words, Exactness,  Typo,       Attribute,  desc(),     Proximity],
            vec![Words, Exactness,  Typo,       Attribute,  Proximity,  desc()],
            vec![Words, Exactness,  Typo,       desc(),     Attribute,  Proximity],
            vec![Words, Exactness,  Typo,       desc(),     Proximity,  Attribute],
            vec![Words, Exactness,  Typo,       Proximity,  Attribute,  desc()],
            vec![Words, Exactness,  Typo,       Proximity,  desc(),     Attribute],
            vec![Words, Proximity,  Attribute,  desc(),     Exactness,  Typo],
            vec![Words, Proximity,  Attribute,  desc(),     Typo,       Exactness],
            vec![Words, Proximity,  Attribute,  Exactness,  desc(),     Typo],
            vec![Words, Proximity,  Attribute,  Exactness,  Typo,       desc()],
            vec![Words, Proximity,  Attribute,  Typo,       desc(),     Exactness],
            vec![Words, Proximity,  Attribute,  Typo,       Exactness,  desc()],
            vec![Words, Proximity,  desc(),     Attribute,  Exactness,  Typo],
            vec![Words, Proximity,  desc(),     Attribute,  Typo,       Exactness],
            vec![Words, Proximity,  desc(),     Exactness,  Attribute,  Typo],
            vec![Words, Proximity,  desc(),     Exactness,  Typo,       Attribute],
            vec![Words, Proximity,  desc(),     Typo,       Attribute,  Exactness],
            vec![Words, Proximity,  desc(),     Typo,       Exactness,  Attribute],
            vec![Words, Proximity,  Exactness,  Attribute,  desc(),     Typo],
            vec![Words, Proximity,  Exactness,  Attribute,  Typo,       desc()],
            vec![Words, Proximity,  Exactness,  desc(),     Attribute,  Typo],
            vec![Words, Proximity,  Exactness,  desc(),     Typo,       Attribute],
            vec![Words, Proximity,  Exactness,  Typo,       Attribute,  desc()],
            vec![Words, Proximity,  Exactness,  Typo,       desc(),     Attribute],
            vec![Words, Proximity,  Typo,       Attribute,  desc(),     Exactness],
            vec![Words, Proximity,  Typo,       Attribute,  Exactness,  desc()],
            vec![Words, Proximity,  Typo,       desc(),     Attribute,  Exactness],
            vec![Words, Proximity,  Typo,       desc(),     Exactness,  Attribute],
            vec![Words, Proximity,  Typo,       Exactness,  Attribute,  desc()],
            vec![Words, Proximity,  Typo,       Exactness,  desc(),     Attribute],
            vec![Words, Typo,       Attribute,  desc(),     Exactness,  Proximity],
            vec![Words, Typo,       Attribute,  desc(),     Proximity,  Exactness],
            vec![Words, Typo,       Attribute,  Exactness,  desc(),     Proximity],
            vec![Words, Typo,       Attribute,  Exactness,  Proximity,  desc()],
            vec![Words, Typo,       Attribute,  Proximity,  desc(),     Exactness],
            vec![Words, Typo,       Attribute,  Proximity,  Exactness,  desc()],
            vec![Words, Typo,       desc(),     Attribute,  Proximity,  Exactness],
            vec![Words, Typo,       desc(),     Exactness,  Attribute,  Proximity],
            vec![Words, Typo,       desc(),     Exactness,  Attribute,  Proximity],
            vec![Words, Typo,       desc(),     Exactness,  Proximity,  Attribute],
            vec![Words, Typo,       desc(),     Proximity,  Attribute,  Exactness],
            vec![Words, Typo,       desc(),     Proximity,  Exactness,  Attribute],
            vec![Words, Typo,       Exactness,  Attribute,  desc(),     Proximity],
            vec![Words, Typo,       Exactness,  Attribute,  Proximity,  desc()],
            vec![Words, Typo,       Exactness,  desc(),     Attribute,  Proximity],
            vec![Words, Typo,       Exactness,  desc(),     Proximity,  Attribute],
            vec![Words, Typo,       Exactness,  Proximity,  Attribute,  desc()],
            vec![Words, Typo,       Exactness,  Proximity,  desc(),     Attribute],
            vec![Words, Typo,       Proximity,  Attribute,  desc(),     Exactness],
            vec![Words, Typo,       Proximity,  Attribute,  Exactness,  desc()],
            vec![Words, Typo,       Proximity,  desc(),     Attribute,  Exactness],
            vec![Words, Typo,       Proximity,  desc(),     Exactness,  Attribute],
            vec![Words, Typo,       Proximity,  Exactness,  Attribute,  desc()],
            vec![Words, Typo,       Proximity,  Exactness,  desc(),     Attribute],
        ]
    };

    let config = IndexerConfig::default();
    for criteria in criteria_mix {
        eprintln!("Testing with criteria order: {:?}", &criteria);
        //update criteria
        let mut wtxn = index.write_txn().unwrap();
        let mut builder = Settings::new(&mut wtxn, &index, &config);
        builder.set_criteria(criteria.iter().map(ToString::to_string).collect());
        builder.execute(|_| ()).unwrap();
        wtxn.commit().unwrap();

        let mut rtxn = index.read_txn().unwrap();

        let mut search = Search::new(&mut rtxn, &index);
        search.query(search::TEST_QUERY);
        search.limit(EXTERNAL_DOCUMENTS_IDS.len());
        search.optional_words(ALLOW_OPTIONAL_WORDS);
        search.authorize_typos(ALLOW_TYPOS);

        let SearchResult { documents_ids, .. } = search.execute().unwrap();

        let expected_external_ids: Vec<_> =
            search::expected_order(&criteria, ALLOW_OPTIONAL_WORDS, ALLOW_TYPOS, &[])
                .into_iter()
                .map(|d| d.id)
                .collect();
        let documents_ids = search::internal_to_external_ids(&index, &documents_ids);

        assert_eq!(documents_ids, expected_external_ids);
    }
}

#[test]
fn criteria_ascdesc() {
    let path = tempfile::tempdir().unwrap();
    let mut options = EnvOpenOptions::new();
    options.map_size(12 * 1024 * 1024); // 10 MB
    let index = Index::new(options, &path).unwrap();

    let mut wtxn = index.write_txn().unwrap();
    let config = IndexerConfig::default();

    let mut builder = Settings::new(&mut wtxn, &index, &config);

    builder.set_sortable_fields(hashset! {
        S("name"),
        S("age"),
    });
    builder.execute(|_| ()).unwrap();

    // index documents
    let config = IndexerConfig { max_memory: Some(10 * 1024 * 1024), ..Default::default() };
    let indexing_config = IndexDocumentsConfig { autogenerate_docids: true, ..Default::default() };
    let builder = IndexDocuments::new(&mut wtxn, &index, &config, indexing_config, |_| ()).unwrap();

    let mut batch_builder = DocumentsBatchBuilder::new(Vec::new());

    (0..ASC_DESC_CANDIDATES_THRESHOLD + 1).for_each(|_| {
        let mut rng = rand::thread_rng();

        let age = rng.gen::<u32>().to_string();
        let name = rng
            .sample_iter(&rand::distributions::Alphanumeric)
            .map(char::from)
            .filter(|c| *c >= 'a' && *c <= 'z')
            .take(10)
            .collect::<String>();

        let json = serde_json::json!({
            "name": name,
            "age": age,
        });

        let object = match json {
            serde_json::Value::Object(object) => object,
            _ => panic!(),
        };

        batch_builder.append_json_object(&object).unwrap();
    });

    let vector = batch_builder.into_inner().unwrap();

    let reader = DocumentsBatchReader::from_reader(Cursor::new(vector)).unwrap();
    let (builder, user_error) = builder.add_documents(reader).unwrap();
    user_error.unwrap();
    builder.execute().unwrap();

    wtxn.commit().unwrap();

    let rtxn = index.read_txn().unwrap();
    let documents = index.all_documents(&rtxn).unwrap().map(|doc| doc.unwrap()).collect::<Vec<_>>();

    for criterion in [Asc(S("name")), Desc(S("name")), Asc(S("age")), Desc(S("age"))] {
        eprintln!("Testing with criterion: {:?}", &criterion);

        let mut wtxn = index.write_txn().unwrap();
        let mut builder = Settings::new(&mut wtxn, &index, &config);
        builder.set_criteria(vec![criterion.to_string()]);
        builder.execute(|_| ()).unwrap();
        wtxn.commit().unwrap();

        let mut rtxn = index.read_txn().unwrap();

        let mut search = Search::new(&mut rtxn, &index);
        search.limit(ASC_DESC_CANDIDATES_THRESHOLD + 1);

        let SearchResult { documents_ids, .. } = search.execute().unwrap();

        let expected_document_ids = match criterion {
            Asc(field_name) if field_name == "name" => {
                documents.iter().sorted_by_key(|(_, obkv)| obkv.get(0).unwrap())
            }
            Desc(field_name) if field_name == "name" => {
                documents.iter().sorted_by_key(|(_, obkv)| Reverse(obkv.get(0).unwrap()))
            }
            Asc(field_name) if field_name == "name" => {
                documents.iter().sorted_by_key(|(_, obkv)| obkv.get(1).unwrap())
            }
            Desc(field_name) if field_name == "name" => {
                documents.iter().sorted_by_key(|(_, obkv)| Reverse(obkv.get(1).unwrap()))
            }
            _ => continue,
        }
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();

        assert_eq!(documents_ids, expected_document_ids);
    }
}
