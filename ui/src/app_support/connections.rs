use super::*;

pub(crate) fn bundled_connector_blurb(locale: Locale, slug: &str) -> Option<&'static str> {
    match (locale, slug) {
        (Locale::En, "biomart") => {
            Some("Query Ensembl BioMart datasets, attributes, and ID translation")
        }
        (Locale::Zh, "biomart") => Some("查询 Ensembl BioMart 数据集、字段并转换基因 ID"),
        (Locale::En, "biorxiv") => Some("Search and retrieve bioRxiv and medRxiv preprints"),
        (Locale::Zh, "biorxiv") => Some("检索 bioRxiv / medRxiv 预印本"),
        (Locale::En, "cancer-models") => {
            Some("Cancer cell models, cBioPortal studies, and gene dependencies")
        }
        (Locale::Zh, "cancer-models") => Some("查询癌症细胞模型、cBioPortal 研究与基因依赖性"),
        (Locale::En, "cellguide") => Some("Cell type ontology, markers, and tissue context"),
        (Locale::Zh, "cellguide") => Some("查询细胞类型、标志基因与组织分布"),
        (Locale::En, "chembl") => Some("ChEMBL compounds, drugs, targets, and bioactivity"),
        (Locale::Zh, "chembl") => Some("检索 ChEMBL 化合物、药物、靶点与生物活性"),
        (Locale::En, "chemistry") => Some("PubChem, ChEBI, BindingDB, and Rhea chemistry data"),
        (Locale::Zh, "chemistry") => Some("查询 PubChem、ChEBI、BindingDB 与 Rhea 化学数据"),
        (Locale::En, "clinical-genomics") => {
            Some("CIViC, ClinGen, and Open Targets clinical evidence")
        }
        (Locale::Zh, "clinical-genomics") => Some("查询 CIViC、ClinGen 与 Open Targets 临床证据"),
        (Locale::En, "clinical-trials") => Some("Search and inspect ClinicalTrials.gov studies"),
        (Locale::Zh, "clinical-trials") => Some("检索并查看 ClinicalTrials.gov 试验"),
        (Locale::En, "drug-regulatory") => {
            Some("FDA drug labels, applications, and pharmacologic classes")
        }
        (Locale::Zh, "drug-regulatory") => Some("查询 FDA 药品标签、申请与药理分类"),
        (Locale::En, "expression") => Some("GTEx and PanglaoDB expression and eQTLs"),
        (Locale::Zh, "expression") => Some("查询 GTEx / PanglaoDB 表达与 eQTL"),
        (Locale::En, "genes-ontologies") => Some("UniProt, GO, KEGG, and Reactome annotations"),
        (Locale::Zh, "genes-ontologies") => Some("查询 UniProt、GO、KEGG 与 Reactome 注释"),
        (Locale::En, "genomes") => Some("Ensembl and UCSC sequences, tracks, and variant effect"),
        (Locale::Zh, "genomes") => Some("查询 Ensembl / UCSC 序列、轨道与变异效应"),
        (Locale::En, "human-genetics") => Some("GWAS, eQTL, and PheWAS associations"),
        (Locale::Zh, "human-genetics") => Some("查询 GWAS、eQTL 与 PheWAS 关联"),
        (Locale::En, "literature") => Some("OpenAlex and arXiv papers, authors, and citations"),
        (Locale::Zh, "literature") => Some("检索 OpenAlex / arXiv 论文、作者与引用"),
        (Locale::En, "omics-archives") => {
            Some("GEO, ArrayExpress, PRIDE, and other omics archives")
        }
        (Locale::Zh, "omics-archives") => Some("检索 GEO、ArrayExpress、PRIDE 等组学档案"),
        (Locale::En, "protein-annotation") => Some("InterPro, Pfam, STRING, and Protein Atlas"),
        (Locale::Zh, "protein-annotation") => Some("查询 InterPro、Pfam、STRING 与 Protein Atlas"),
        (Locale::En, "pubmed") => Some("Search PubMed articles, metadata, and full text"),
        (Locale::Zh, "pubmed") => Some("检索 PubMed 文献、元数据与全文"),
        (Locale::En, "regulation") => Some("ENCODE, JASPAR, and UniBind regulatory data"),
        (Locale::Zh, "regulation") => Some("查询 ENCODE、JASPAR 与 UniBind 调控数据"),
        (Locale::En, "research-resources") => Some("Antibodies and research grants"),
        (Locale::Zh, "research-resources") => Some("检索抗体试剂与科研基金"),
        (Locale::En, "rna") => Some("Rfam RNA families, alignments, and structures"),
        (Locale::Zh, "rna") => Some("查询 Rfam RNA 家族、比对与结构"),
        (Locale::En, "structures-interactions") => {
            Some("PDB, AlphaFold, EMDB, and protein interactions")
        }
        (Locale::Zh, "structures-interactions") => Some("查询 PDB、AlphaFold、EMDB 与蛋白互作"),
        (Locale::En, "variants") => Some("ClinVar, dbSNP, CADD, and structural variants"),
        (Locale::Zh, "variants") => Some("查询 ClinVar、dbSNP、CADD 与结构变异"),
        (Locale::En, "zinc") => Some("Search the ZINC purchasable compound library"),
        (Locale::Zh, "zinc") => Some("检索 ZINC 可购买化合物库"),
        _ => None,
    }
}

/// One-line capability text under an MCP tool name, matching the UI locale.
pub(crate) fn mcp_tool_brief(locale: Locale, name: &str, raw: &str) -> String {
    match locale {
        Locale::Zh => {
            if let Some(zh) = bundled_tool_brief_zh(name) {
                return zh.to_string();
            }
            let brief = first_brief_line(raw);
            if !brief.is_empty() {
                brief
            } else {
                humanize_tool_name(name)
            }
        }
        Locale::En => {
            let brief = first_brief_line(raw);
            if !brief.is_empty() {
                brief
            } else {
                humanize_tool_name(name)
            }
        }
    }
}

fn bundled_tool_brief_zh(name: &str) -> Option<&'static str> {
    Some(match name {
        "batch_translate" => "批量把一批标识符从一种类型转换成另一种",
        "get_data" => "按指定字段和筛选条件从 BioMart 取数",
        "get_translation" => "把单个标识符从一种类型转换成另一种",
        "list_all_attributes" => "列出某个数据集的全部可用字段",
        "list_common_attributes" => "列出某个数据集最常用的字段",
        "list_datasets" => "列出某个 BioMart 下的全部数据集",
        "list_filters" => "列出某个数据集可用的筛选条件",
        "list_marts" => "列出 Ensembl 上所有可用的 BioMart 数据库",
        "get_categories" => "列出 bioRxiv / medRxiv 的学科分类",
        "get_content_statistics" => "查看预印本平台的内容统计",
        "get_preprint" => "按 DOI 或 ID 获取一篇预印本",
        "get_usage_statistics" => "查看预印本的下载与使用统计",
        "search_by_funder" => "按资助方检索预印本",
        "search_preprints" => "检索 bioRxiv / medRxiv 预印本",
        "search_published_preprints" => "检索已正式发表的预印本",
        "cbioportal_clinical_attributes" => "查看 cBioPortal 研究的临床属性",
        "cbioportal_cna_in_gene" => "查询某基因在指定研究中的拷贝数改变",
        "cbioportal_get_study" => "获取一项 cBioPortal 研究的详情",
        "cbioportal_list_studies" => "列出或检索 cBioPortal 研究",
        "cbioportal_mutation_frequency" => "统计某基因在研究中的突变频率",
        "cbioportal_mutations_in_gene" => "查询某基因在指定研究中的突变",
        "gene_dependencies" => "查询癌细胞模型中的基因依赖性",
        "get_model" => "按编号或名称获取一个癌症细胞模型",
        "list_models" => "按组织等条件列出癌症细胞模型",
        "search_genes" => "检索癌症模型相关基因",
        "search_models" => "按关键词检索癌症细胞模型",
        "get_cell_tissues" => "查看某细胞类型出现在哪些组织",
        "get_cell_type_info" => "获取细胞类型的定义、同义词与基本信息",
        "get_marker_genes" => "获取某细胞类型的标志基因",
        "get_source_data" => "查看该细胞类型注释的来源数据",
        "search_cell_types" => "按名称或同义词检索细胞类型",
        "compound_search" => "在 ChEMBL 中检索化合物",
        "drug_search" => "在 ChEMBL 中检索药物",
        "get_admet" => "查询化合物的 ADMET 性质",
        "get_bioactivity" => "查询化合物或靶点的生物活性数据",
        "get_mechanism" => "查询药物的作用机制",
        "target_search" => "在 ChEMBL 中检索靶点",
        "bindingdb_ligands_by_target" => "按靶点查找 BindingDB 中的配体",
        "bindingdb_targets_by_compound" => "按化合物查找 BindingDB 中的靶点",
        "chebi_get_entity" => "获取一条 ChEBI 化学实体",
        "chebi_get_ontology" => "查看 ChEBI 实体的本体关系",
        "chebi_search" => "检索 ChEBI 化学实体",
        "pubchem_get_bioassay_summary" => "获取 PubChem 化合物的生物实验摘要",
        "pubchem_get_compounds" => "按 CID 获取 PubChem 化合物",
        "pubchem_get_safety" => "查询 PubChem 化合物的安全信息",
        "pubchem_search_compounds" => "按名称等检索 PubChem 化合物",
        "pubchem_similarity_search" => "按结构相似性检索 PubChem 化合物",
        "rhea_get_reaction" => "获取一条 Rhea 生化反应",
        "rhea_search_reactions" => "检索 Rhea 生化反应",
        "civic_gene_variants" => "查询 CIViC 中某基因的变异",
        "civic_get_assertion" => "获取一条 CIViC 临床断言",
        "civic_get_evidence_item" => "获取一条 CIViC 证据条目",
        "civic_get_molecular_profile" => "获取 CIViC 分子图谱",
        "civic_get_variant" => "获取一条 CIViC 变异记录",
        "civic_search_assertions" => "检索 CIViC 临床断言",
        "civic_search_diseases" => "检索 CIViC 疾病",
        "civic_search_evidence" => "检索 CIViC 证据",
        "civic_search_genes" => "检索 CIViC 基因",
        "civic_search_molecular_profiles" => "检索 CIViC 分子图谱",
        "civic_search_therapies" => "检索 CIViC 疗法",
        "civic_search_variants" => "检索 CIViC 变异",
        "clingen_actionability" => "查询 ClinGen 基因可操作性评估",
        "clingen_dosage_sensitivity" => "查询 ClinGen 剂量敏感性",
        "clingen_gene_validity" => "查询 ClinGen 基因-疾病有效性",
        "clingen_variant_classifications" => "查询 ClinGen 变异分级",
        "open_targets_disease_drugs" => "查询 Open Targets 疾病相关药物",
        "open_targets_disease_targets" => "查询 Open Targets 疾病相关靶点",
        "open_targets_drug" => "获取 Open Targets 药物信息",
        "open_targets_graphql" => "用 GraphQL 查询 Open Targets",
        "analyze_endpoints" => "分析临床试验的主要与次要终点",
        "get_trial_details" => "获取一项临床试验的详细信息",
        "search_by_eligibility" => "按入组条件检索临床试验",
        "search_by_sponsor" => "按申办方检索临床试验",
        "search_investigators" => "检索临床试验研究者",
        "search_trials" => "检索 ClinicalTrials.gov 试验",
        "count_drug_applications" => "统计 FDA 药品申请数量",
        "get_drug_application" => "获取一份 FDA 药品申请",
        "get_drug_statistics" => "查看 FDA 药品申请统计",
        "get_generic_equivalents" => "查询品牌药的仿制药等效产品",
        "list_pharmacologic_classes" => "列出 FDA 药理分类",
        "search_drug_applications" => "检索 FDA 药品申请",
        "search_drug_labels" => "检索 FDA 药品标签",
        "gtex_calculate_eqtl" => "计算 GTEx 中的 eQTL 关联",
        "gtex_dataset_info" => "查看 GTEx 数据集信息",
        "gtex_eqtl_genes" => "查询具有 eQTL 的基因",
        "gtex_expression_summary" => "查看 GTEx 基因表达摘要",
        "gtex_gene_expression" => "查询某基因在 GTEx 组织中的表达",
        "gtex_median_expression" => "查询 GTEx 中位表达量",
        "gtex_multi_tissue_eqtls" => "查询跨组织 eQTL",
        "gtex_resolve_genes" => "把基因名解析为 GTEx 标识",
        "gtex_sample_info" => "查看 GTEx 样本信息",
        "gtex_single_tissue_eqtls" => "查询单组织 eQTL",
        "gtex_tissue_sites" => "列出 GTEx 组织部位",
        "gtex_top_expressed_genes" => "查看某组织中表达最高的基因",
        "panglaodb_cell_types_for_gene" => "按基因查找 PanglaoDB 细胞类型",
        "panglaodb_marker_genes" => "查询 PanglaoDB 标志基因",
        "panglaodb_options" => "查看 PanglaoDB 可选物种与组织",
        "get_go_annotations" => "获取基因的 GO 注释",
        "get_kegg_entries" => "获取 KEGG 条目",
        "get_ontology_term" => "获取一条本体术语",
        "get_uniprot_entries" => "获取 UniProt 蛋白条目",
        "link_kegg_ids" => "在 KEGG 标识之间做关联映射",
        "list_ontologies" => "列出可用的本体",
        "map_reactome_pathways" => "把基因映射到 Reactome 通路",
        "query_genes" => "查询基因注释与标识",
        "search_kegg" => "检索 KEGG 条目",
        "search_ontology_terms" => "检索本体术语",
        "ensembl_homology" => "查询 Ensembl 同源基因",
        "ensembl_lookup" => "按 ID 查询 Ensembl 基因或转录本",
        "ensembl_overlap_region" => "查询与基因组区间重叠的特征",
        "ensembl_sequence" => "获取 Ensembl 序列",
        "ensembl_vep_variant" => "用 VEP 注释变异效应",
        "ensembl_xrefs" => "查询 Ensembl 交叉引用",
        "ucsc_chrom_sizes" => "获取 UCSC 染色体长度",
        "ucsc_conservation" => "查询 UCSC 保守性轨道",
        "ucsc_list_tracks" => "列出 UCSC 可用轨道",
        "ucsc_tfbs_clusters" => "查询 UCSC 转录因子结合簇",
        "ucsc_track_data" => "获取一条 UCSC 轨道的数据",
        "eqtl_associations" => "查询 eQTL 关联结果",
        "eqtl_list_datasets" => "列出 eQTL 数据集",
        "gwas_associations_for_gene" => "按基因查询 GWAS 关联",
        "gwas_associations_for_trait" => "按性状查询 GWAS 关联",
        "gwas_associations_for_variant" => "按变异查询 GWAS 关联",
        "gwas_get_study" => "获取一项 GWAS 研究",
        "gwas_get_variant" => "获取一个 GWAS 变异",
        "gwas_search_studies" => "检索 GWAS 研究",
        "gwas_search_traits" => "检索 GWAS 性状",
        "phewas_finngen_gene" => "按基因查询 FinnGen PheWAS",
        "phewas_instances" => "查看 PheWAS 数据实例",
        "phewas_list_phenotypes" => "列出 PheWAS 表型",
        "phewas_search_phenotypes" => "检索 PheWAS 表型",
        "phewas_variant" => "按变异查询 PheWAS 结果",
        "arxiv_get_papers" => "按 arXiv ID 获取论文",
        "arxiv_search" => "检索 arXiv 预印本",
        "openalex_citations" => "查看一篇 OpenAlex 文献的被引",
        "openalex_get_author" => "获取一位 OpenAlex 作者",
        "openalex_get_work" => "获取一篇 OpenAlex 文献",
        "openalex_references" => "查看一篇 OpenAlex 文献的参考文献",
        "openalex_search_authors" => "检索 OpenAlex 作者",
        "openalex_search_works" => "检索 OpenAlex 文献",
        "openalex_venue_info" => "查看 OpenAlex 期刊或会议信息",
        "arrayexpress_get_experiment" => "获取一项 ArrayExpress 实验",
        "arrayexpress_get_experiment_files" => "列出 ArrayExpress 实验文件",
        "arrayexpress_get_experiment_samples" => "查看 ArrayExpress 实验样本",
        "arrayexpress_search_experiments" => "检索 ArrayExpress 实验",
        "geo_get_series" => "获取一个 GEO 系列的元数据",
        "geo_search_series" => "检索 GEO 表达系列",
        "metabolights_get_studies" => "获取 MetaboLights 研究",
        "metabolights_get_study_files" => "列出 MetaboLights 研究文件",
        "metabolights_list_studies" => "列出 MetaboLights 研究",
        "metabolights_search_data_files" => "检索 MetaboLights 数据文件",
        "mgnify_get_studies" => "获取 MGnify 微生物组研究",
        "mgnify_get_study_analyses" => "查看 MGnify 研究的分析",
        "mgnify_search_studies" => "检索 MGnify 研究",
        "pride_find_projects_for_protein" => "按蛋白查找 PRIDE 项目",
        "pride_get_projects" => "获取 PRIDE 蛋白质组项目",
        "pride_search_project_proteins" => "在 PRIDE 项目中检索蛋白",
        "pride_search_projects" => "检索 PRIDE 项目",
        "get_domain_architecture" => "获取蛋白的结构域架构",
        "get_interpro_entry" => "获取一条 InterPro 条目",
        "get_pfam_clan" => "获取一个 Pfam 家族组",
        "get_pfam_family_proteins" => "列出某 Pfam 家族的蛋白",
        "get_pfam_family_proteomes" => "列出某 Pfam 家族覆盖的蛋白质组",
        "get_protein_atlas_gene" => "获取 Protein Atlas 基因信息",
        "get_string_best_similarity_hits" => "查询 STRING 最佳序列相似命中",
        "get_string_network" => "获取 STRING 蛋白互作网络",
        "get_string_similarity_scores" => "查询 STRING 序列相似性分数",
        "map_string_ids" => "把基因符号映射为 STRING 编号",
        "search_interpro_entries" => "检索 InterPro 条目",
        "search_pfam_clans" => "检索 Pfam 家族组",
        "search_protein_atlas" => "检索 Protein Atlas 基因",
        "convert_article_ids" => "在 PMID、PMCID、DOI 之间转换文献编号",
        "find_related_articles" => "查找与给定文献相关的文章或资源",
        "get_article_metadata" => "按 PMID 获取文献标题、摘要与元数据",
        "get_copyright_status" => "查询文献版权与开放获取许可",
        "get_full_text_article" => "从 PMC 获取文献全文",
        "lookup_article_by_citation" => "按题名、作者等引文信息反查文献",
        "search_articles" => "按关键词检索 PubMed 生物医学文献",
        "encode_get_biosample" => "获取一项 ENCODE 生物样本",
        "encode_get_experiment" => "获取一项 ENCODE 实验",
        "encode_get_file" => "获取一个 ENCODE 文件记录",
        "encode_list_files" => "列出符合条件的 ENCODE 文件",
        "encode_search_biosamples" => "检索 ENCODE 生物样本",
        "encode_search_experiments" => "检索 ENCODE 实验",
        "jaspar_get_matrix" => "获取一个 JASPAR 转录因子矩阵",
        "jaspar_list_collections" => "列出 JASPAR 矩阵集合",
        "jaspar_list_matrices" => "列出 JASPAR 矩阵",
        "jaspar_list_releases" => "列出 JASPAR 版本",
        "jaspar_list_species" => "列出 JASPAR 覆盖的物种",
        "jaspar_list_taxa" => "列出 JASPAR 分类群",
        "jaspar_matrix_versions" => "查看同一基质的各版本",
        "unibind_get_dataset" => "获取 UniBind 转录因子数据集",
        "unibind_search_tfbs" => "检索 UniBind 转录因子结合位点",
        "unibind_tfbs_in_region" => "查询基因组区间内的 UniBind 结合位点",
        "find_antibodies_by_catalog" => "按目录号查找抗体",
        "get_antibody" => "获取一条抗体登记记录",
        "get_antibody_registry_stats" => "查看抗体登记库统计",
        "search_antibodies" => "检索抗体试剂",
        "search_grants" => "检索科研基金项目",
        "accession_to_id" => "把 Rfam 登录号转换成内部编号",
        "get_covariance_model" => "获取 RNA 家族的协方差模型",
        "get_family" => "获取一个 Rfam RNA 家族",
        "get_seed_alignment" => "获取 RNA 家族的种子比对",
        "get_sequence_regions" => "查询 RNA 家族的序列区间",
        "get_structure_mapping" => "获取 RNA 家族的结构映射",
        "get_tree" => "获取 RNA 家族的进化树",
        "id_to_accession" => "把内部编号转换成 Rfam 登录号",
        "search_sequence" => "按序列检索 RNA 家族",
        "alphafold_check_coverage" => "检查 UniProt 条目的 AlphaFold 覆盖",
        "alphafold_get_prediction" => "获取 AlphaFold 结构预测",
        "complexportal_get_complexes" => "获取 Complex Portal 复合物",
        "complexportal_search_by_participant" => "按组分检索蛋白复合物",
        "emdb_get_entries" => "获取 EMDB 电镜密度图条目",
        "emdb_get_entry_section" => "获取 EMDB 条目的某一章节",
        "emdb_get_validation" => "查看 EMDB 条目的校验信息",
        "emdb_search_entries" => "检索 EMDB 电镜条目",
        "intact_build_network" => "构建 IntAct 蛋白互作网络",
        "intact_fetch_interactions" => "获取 IntAct 互作记录",
        "intact_get_interaction_details" => "获取一条 IntAct 互作的详情",
        "intact_get_interactor" => "获取一个 IntAct 互作蛋白",
        "pdb_get_entities" => "获取 PDB 结构中的分子实体",
        "pdb_get_ligands" => "获取 PDB 结构中的配体",
        "pdb_get_structures" => "按 PDB ID 获取结构记录",
        "pdb_search_structures" => "检索 PDB 三维结构",
        "cadd_position_scores" => "查询某基因组位点的 CADD 评分",
        "cadd_range_scores" => "查询基因组区间内的 CADD 评分",
        "cadd_variant_score" => "查询单个变异的 CADD 评分",
        "clinvar_get_records" => "获取 ClinVar 记录",
        "clinvar_search" => "检索 ClinVar 变异与表型",
        "clinvar_variant_by_rsid" => "按 rsID 查询 ClinVar 变异",
        "clinvar_variants" => "查询 ClinVar 变异",
        "dbsnp_get_rsids" => "按 rsID 获取 dbSNP 记录",
        "dbsnp_search_by_region" => "按基因组区间检索 dbSNP",
        "gene_constraint" => "查询基因约束指标",
        "gene_variants" => "查询某基因的变异",
        "get_structural_variant" => "获取一条结构变异",
        "get_variant" => "获取一条变异记录",
        "liftover_variant" => "把变异坐标转换到另一个基因组版本",
        "mitochondrial_variants" => "查询线粒体变异",
        "region_variants" => "查询基因组区间内的变异",
        "search_variants" => "检索变异记录",
        "structural_variants" => "查询结构变异",
        "zinc_get_3d" => "获取 ZINC 化合物的三维结构",
        "zinc_random_sample" => "从 ZINC 库随机抽取可购买化合物",
        "zinc_search_by_id" => "按 ZINC 编号查找可购买化合物",
        "zinc_search_by_smiles" => "按 SMILES 检索 ZINC 化合物",
        "zinc_search_by_supplier" => "按供应商检索 ZINC 化合物",
        _ => return None,
    })
}

fn first_brief_line(raw: &str) -> String {
    const MAX: usize = 140;
    let text = raw.replace('\r', "");
    let para = text
        .split("\n\n")
        .map(str::trim)
        .find(|p| !p.is_empty())
        .unwrap_or("");
    let mut combined = String::new();
    for line in para.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if combined.is_empty() {
            combined.push_str(line);
        } else {
            combined.push(' ');
            combined.push_str(line);
        }
        if ends_brief_sentence(&combined) || combined.chars().count() >= MAX {
            break;
        }
    }
    let brief = first_sentence(&combined);
    if brief.chars().count() <= MAX {
        return brief;
    }
    let mut truncated: String = brief.chars().take(MAX.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

fn first_sentence(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'.' && (i + 1 == bytes.len() || bytes[i + 1].is_ascii_whitespace()) {
            return trimmed[..=i].trim().to_string();
        }
        i += 1;
    }
    trimmed.to_string()
}

fn ends_brief_sentence(text: &str) -> bool {
    let t = text.trim_end();
    t.ends_with('.') || t.ends_with('。') || t.ends_with('!') || t.ends_with('?')
}

fn humanize_tool_name(name: &str) -> String {
    let mut words = name
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect::<Vec<_>>();
    if let Some(first) = words.first_mut() {
        let mut chars = first.chars();
        if let Some(c) = chars.next() {
            *first = c.to_uppercase().collect::<String>() + chars.as_str();
        }
    }
    words.join(" ")
}

#[cfg(test)]
mod connector_blurb_tests {
    use super::{bundled_connector_blurb, bundled_tool_brief_zh, first_brief_line, mcp_tool_brief};
    use crate::i18n::Locale;

    const DOMAIN_SLUGS: &[&str] = &[
        "biomart",
        "biorxiv",
        "cancer-models",
        "cellguide",
        "chembl",
        "chemistry",
        "clinical-genomics",
        "clinical-trials",
        "drug-regulatory",
        "expression",
        "genes-ontologies",
        "genomes",
        "human-genetics",
        "literature",
        "omics-archives",
        "protein-annotation",
        "pubmed",
        "regulation",
        "research-resources",
        "rna",
        "structures-interactions",
        "variants",
        "zinc",
    ];

    const BUNDLED_TOOLS: &[&str] = &[
        "batch_translate",
        "get_data",
        "get_translation",
        "list_all_attributes",
        "list_common_attributes",
        "list_datasets",
        "list_filters",
        "list_marts",
        "get_categories",
        "get_content_statistics",
        "get_preprint",
        "get_usage_statistics",
        "search_by_funder",
        "search_preprints",
        "search_published_preprints",
        "cbioportal_clinical_attributes",
        "cbioportal_cna_in_gene",
        "cbioportal_get_study",
        "cbioportal_list_studies",
        "cbioportal_mutation_frequency",
        "cbioportal_mutations_in_gene",
        "gene_dependencies",
        "get_model",
        "list_models",
        "search_genes",
        "search_models",
        "get_cell_tissues",
        "get_cell_type_info",
        "get_marker_genes",
        "get_source_data",
        "search_cell_types",
        "compound_search",
        "drug_search",
        "get_admet",
        "get_bioactivity",
        "get_mechanism",
        "target_search",
        "bindingdb_ligands_by_target",
        "bindingdb_targets_by_compound",
        "chebi_get_entity",
        "chebi_get_ontology",
        "chebi_search",
        "pubchem_get_bioassay_summary",
        "pubchem_get_compounds",
        "pubchem_get_safety",
        "pubchem_search_compounds",
        "pubchem_similarity_search",
        "rhea_get_reaction",
        "rhea_search_reactions",
        "civic_gene_variants",
        "civic_get_assertion",
        "civic_get_evidence_item",
        "civic_get_molecular_profile",
        "civic_get_variant",
        "civic_search_assertions",
        "civic_search_diseases",
        "civic_search_evidence",
        "civic_search_genes",
        "civic_search_molecular_profiles",
        "civic_search_therapies",
        "civic_search_variants",
        "clingen_actionability",
        "clingen_dosage_sensitivity",
        "clingen_gene_validity",
        "clingen_variant_classifications",
        "open_targets_disease_drugs",
        "open_targets_disease_targets",
        "open_targets_drug",
        "open_targets_graphql",
        "analyze_endpoints",
        "get_trial_details",
        "search_by_eligibility",
        "search_by_sponsor",
        "search_investigators",
        "search_trials",
        "count_drug_applications",
        "get_drug_application",
        "get_drug_statistics",
        "get_generic_equivalents",
        "list_pharmacologic_classes",
        "search_drug_applications",
        "search_drug_labels",
        "gtex_calculate_eqtl",
        "gtex_dataset_info",
        "gtex_eqtl_genes",
        "gtex_expression_summary",
        "gtex_gene_expression",
        "gtex_median_expression",
        "gtex_multi_tissue_eqtls",
        "gtex_resolve_genes",
        "gtex_sample_info",
        "gtex_single_tissue_eqtls",
        "gtex_tissue_sites",
        "gtex_top_expressed_genes",
        "panglaodb_cell_types_for_gene",
        "panglaodb_marker_genes",
        "panglaodb_options",
        "get_go_annotations",
        "get_kegg_entries",
        "get_ontology_term",
        "get_uniprot_entries",
        "link_kegg_ids",
        "list_ontologies",
        "map_reactome_pathways",
        "query_genes",
        "search_kegg",
        "search_ontology_terms",
        "ensembl_homology",
        "ensembl_lookup",
        "ensembl_overlap_region",
        "ensembl_sequence",
        "ensembl_vep_variant",
        "ensembl_xrefs",
        "ucsc_chrom_sizes",
        "ucsc_conservation",
        "ucsc_list_tracks",
        "ucsc_tfbs_clusters",
        "ucsc_track_data",
        "eqtl_associations",
        "eqtl_list_datasets",
        "gwas_associations_for_gene",
        "gwas_associations_for_trait",
        "gwas_associations_for_variant",
        "gwas_get_study",
        "gwas_get_variant",
        "gwas_search_studies",
        "gwas_search_traits",
        "phewas_finngen_gene",
        "phewas_instances",
        "phewas_list_phenotypes",
        "phewas_search_phenotypes",
        "phewas_variant",
        "arxiv_get_papers",
        "arxiv_search",
        "openalex_citations",
        "openalex_get_author",
        "openalex_get_work",
        "openalex_references",
        "openalex_search_authors",
        "openalex_search_works",
        "openalex_venue_info",
        "arrayexpress_get_experiment",
        "arrayexpress_get_experiment_files",
        "arrayexpress_get_experiment_samples",
        "arrayexpress_search_experiments",
        "geo_get_series",
        "geo_search_series",
        "metabolights_get_studies",
        "metabolights_get_study_files",
        "metabolights_list_studies",
        "metabolights_search_data_files",
        "mgnify_get_studies",
        "mgnify_get_study_analyses",
        "mgnify_search_studies",
        "pride_find_projects_for_protein",
        "pride_get_projects",
        "pride_search_project_proteins",
        "pride_search_projects",
        "get_domain_architecture",
        "get_interpro_entry",
        "get_pfam_clan",
        "get_pfam_family_proteins",
        "get_pfam_family_proteomes",
        "get_protein_atlas_gene",
        "get_string_best_similarity_hits",
        "get_string_network",
        "get_string_similarity_scores",
        "map_string_ids",
        "search_interpro_entries",
        "search_pfam_clans",
        "search_protein_atlas",
        "convert_article_ids",
        "find_related_articles",
        "get_article_metadata",
        "get_copyright_status",
        "get_full_text_article",
        "lookup_article_by_citation",
        "search_articles",
        "encode_get_biosample",
        "encode_get_experiment",
        "encode_get_file",
        "encode_list_files",
        "encode_search_biosamples",
        "encode_search_experiments",
        "jaspar_get_matrix",
        "jaspar_list_collections",
        "jaspar_list_matrices",
        "jaspar_list_releases",
        "jaspar_list_species",
        "jaspar_list_taxa",
        "jaspar_matrix_versions",
        "unibind_get_dataset",
        "unibind_search_tfbs",
        "unibind_tfbs_in_region",
        "find_antibodies_by_catalog",
        "get_antibody",
        "get_antibody_registry_stats",
        "search_antibodies",
        "search_grants",
        "accession_to_id",
        "get_covariance_model",
        "get_family",
        "get_seed_alignment",
        "get_sequence_regions",
        "get_structure_mapping",
        "get_tree",
        "id_to_accession",
        "search_sequence",
        "alphafold_check_coverage",
        "alphafold_get_prediction",
        "complexportal_get_complexes",
        "complexportal_search_by_participant",
        "emdb_get_entries",
        "emdb_get_entry_section",
        "emdb_get_validation",
        "emdb_search_entries",
        "intact_build_network",
        "intact_fetch_interactions",
        "intact_get_interaction_details",
        "intact_get_interactor",
        "pdb_get_entities",
        "pdb_get_ligands",
        "pdb_get_structures",
        "pdb_search_structures",
        "cadd_position_scores",
        "cadd_range_scores",
        "cadd_variant_score",
        "clinvar_get_records",
        "clinvar_search",
        "clinvar_variant_by_rsid",
        "clinvar_variants",
        "dbsnp_get_rsids",
        "dbsnp_search_by_region",
        "gene_constraint",
        "gene_variants",
        "get_structural_variant",
        "get_variant",
        "liftover_variant",
        "mitochondrial_variants",
        "region_variants",
        "search_variants",
        "structural_variants",
        "zinc_get_3d",
        "zinc_random_sample",
        "zinc_search_by_id",
        "zinc_search_by_smiles",
        "zinc_search_by_supplier",
    ];

    #[test]
    fn every_featured_domain_has_a_capability_line() {
        for slug in DOMAIN_SLUGS {
            assert!(
                bundled_connector_blurb(Locale::Zh, slug).is_some(),
                "missing zh blurb for {slug}"
            );
            assert!(
                bundled_connector_blurb(Locale::En, slug).is_some(),
                "missing en blurb for {slug}"
            );
        }
    }

    #[test]
    fn every_bundled_tool_has_a_chinese_brief() {
        for name in BUNDLED_TOOLS {
            let zh = bundled_tool_brief_zh(name).unwrap_or("");
            assert!(!zh.is_empty(), "missing zh brief for {name}");
            assert!(
                zh.chars().any(|c| !c.is_ascii()),
                "zh brief for {name} is not Chinese: {zh}"
            );
        }
    }

    #[test]
    fn tool_brief_follows_the_ui_locale() {
        let raw = "Search PubMed for biomedical articles matching a query.\n\nIMPORTANT leftover.";
        assert_eq!(
            mcp_tool_brief(Locale::En, "search_articles", raw),
            "Search PubMed for biomedical articles matching a query."
        );
        assert_eq!(
            mcp_tool_brief(Locale::Zh, "search_articles", raw),
            "按关键词检索 PubMed 生物医学文献"
        );
        assert_eq!(
            mcp_tool_brief(Locale::Zh, "custom_tool", "Search Wolai pages"),
            "Search Wolai pages"
        );
        assert_eq!(first_brief_line("Search Wolai pages"), "Search Wolai pages");
    }
}
