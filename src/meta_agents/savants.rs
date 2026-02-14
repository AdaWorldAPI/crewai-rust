//! Pre-built savant agent blueprints for common domains.
//!
//! Savants are expert-level agent blueprints that encode domain knowledge
//! about what skills, tools, and configurations produce the best results
//! for particular types of tasks. The orchestrator uses these blueprints
//! to spawn agents on demand.
//!
//! Each savant function returns an `AgentBlueprint` configured for
//! its domain, which can be further customized before spawning.

use super::types::{AgentBlueprint, SavantDomain, SkillDescriptor};

/// Create a research savant blueprint.
///
/// Expert at finding, synthesizing, and validating information from
/// multiple sources including web search, academic papers, and databases.
pub fn research_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Senior Research Analyst",
        "Find accurate, comprehensive information from authoritative sources and synthesize it into actionable insights",
        "You are a world-class research analyst with expertise in information retrieval, source validation, \
         and knowledge synthesis. You systematically explore topics from multiple angles, cross-reference \
         findings, and present results with proper attribution. You distinguish between facts and speculation.",
        llm,
        SavantDomain::Research,
    )
    .with_skill(
        SkillDescriptor::new("web_research", "Web Research", "Search the web for current information")
            .with_tags(vec!["research".into(), "web".into(), "search".into(), "information".into()])
            .with_tools(vec!["SerperDevTool".into(), "BraveSearchTool".into(), "ScrapeWebsiteTool".into()])
    )
    .with_skill(
        SkillDescriptor::new("data_synthesis", "Data Synthesis", "Combine information from multiple sources into coherent summaries")
            .with_tags(vec!["synthesis".into(), "analysis".into(), "summary".into()])
    )
    .with_skill(
        SkillDescriptor::new("fact_checking", "Fact Checking", "Verify claims against authoritative sources")
            .with_tags(vec!["verification".into(), "facts".into(), "accuracy".into()])
    )
    .with_tools(vec![
        "SerperDevTool".into(),
        "BraveSearchTool".into(),
        "ScrapeWebsiteTool".into(),
    ])
    .with_delegation()
}

/// Create an engineering savant blueprint.
///
/// Expert at software architecture, code generation, debugging, and
/// code review across multiple languages and frameworks.
pub fn engineering_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Staff Software Engineer",
        "Design, implement, review, and debug software systems with high code quality and maintainability",
        "You are a staff-level software engineer with deep expertise in multiple programming languages, \
         software architecture patterns, and engineering best practices. You write clean, well-tested code \
         and can debug complex issues systematically. You understand performance, security, and scalability.",
        llm,
        SavantDomain::Engineering,
    )
    .with_skill(
        SkillDescriptor::new("code_generation", "Code Generation", "Write production-quality code in multiple languages")
            .with_tags(vec!["code".into(), "programming".into(), "implementation".into(), "development".into()])
            .with_tools(vec!["FileReadTool".into(), "FileWriterTool".into(), "DirectoryReadTool".into()])
    )
    .with_skill(
        SkillDescriptor::new("code_review", "Code Review", "Review code for bugs, security issues, and best practices")
            .with_tags(vec!["review".into(), "quality".into(), "bugs".into(), "security".into()])
            .with_tools(vec!["FileReadTool".into()])
    )
    .with_skill(
        SkillDescriptor::new("debugging", "Debugging", "Systematically diagnose and fix software bugs")
            .with_tags(vec!["debug".into(), "fix".into(), "troubleshoot".into(), "error".into()])
    )
    .with_skill(
        SkillDescriptor::new("architecture", "Architecture Design", "Design scalable software architectures")
            .with_tags(vec!["architecture".into(), "design".into(), "system".into(), "scalable".into()])
    )
    .with_tools(vec![
        "FileReadTool".into(),
        "FileWriterTool".into(),
        "DirectoryReadTool".into(),
    ])
}

/// Create a data analysis savant blueprint.
///
/// Expert at data processing, statistical analysis, visualization,
/// and deriving insights from structured and unstructured data.
pub fn data_analysis_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Senior Data Analyst",
        "Analyze data to extract patterns, trends, and actionable insights using statistical methods",
        "You are a senior data analyst with expertise in statistics, data visualization, and machine \
         learning. You can work with structured data (CSV, JSON, SQL) and unstructured data (text, logs). \
         You communicate findings clearly with appropriate visualizations and confidence intervals.",
        llm,
        SavantDomain::DataAnalysis,
    )
    .with_skill(
        SkillDescriptor::new("data_processing", "Data Processing", "Clean, transform, and prepare data for analysis")
            .with_tags(vec!["data".into(), "processing".into(), "ETL".into(), "cleaning".into()])
    )
    .with_skill(
        SkillDescriptor::new("statistical_analysis", "Statistical Analysis", "Apply statistical methods to derive insights")
            .with_tags(vec!["statistics".into(), "analysis".into(), "correlation".into(), "regression".into()])
    )
    .with_skill(
        SkillDescriptor::new("data_visualization", "Data Visualization", "Create clear, informative data visualizations")
            .with_tags(vec!["visualization".into(), "charts".into(), "graphs".into(), "dashboard".into()])
    )
    .with_tools(vec!["FileReadTool".into()])
}

/// Create a content creation savant blueprint.
///
/// Expert at writing, editing, and formatting content across multiple
/// formats including technical documentation, marketing copy, and reports.
pub fn content_creation_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Senior Content Strategist",
        "Create compelling, well-structured content tailored to specific audiences and objectives",
        "You are a senior content strategist with expertise in technical writing, copywriting, and \
         editorial processes. You adapt tone and style for different audiences, maintain consistency, \
         and ensure clarity. You understand SEO, accessibility, and content architecture.",
        llm,
        SavantDomain::ContentCreation,
    )
    .with_skill(
        SkillDescriptor::new("technical_writing", "Technical Writing", "Write clear technical documentation and guides")
            .with_tags(vec!["writing".into(), "documentation".into(), "technical".into(), "docs".into()])
    )
    .with_skill(
        SkillDescriptor::new("copywriting", "Copywriting", "Write persuasive marketing and promotional content")
            .with_tags(vec!["marketing".into(), "copy".into(), "persuasion".into(), "branding".into()])
    )
    .with_skill(
        SkillDescriptor::new("editing", "Editing & Proofreading", "Review and improve written content for clarity and accuracy")
            .with_tags(vec!["editing".into(), "proofreading".into(), "grammar".into(), "style".into()])
    )
}

/// Create a planning savant blueprint.
///
/// Expert at strategic planning, task decomposition, project management,
/// and resource allocation.
pub fn planning_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Strategic Planning Director",
        "Decompose complex objectives into actionable plans with clear milestones and dependencies",
        "You are a strategic planning director with expertise in project management, task decomposition, \
         and resource allocation. You break down complex goals into manageable tasks, identify dependencies, \
         estimate effort, and create realistic timelines. You consider risks and contingencies.",
        llm,
        SavantDomain::Planning,
    )
    .with_skill(
        SkillDescriptor::new("task_decomposition", "Task Decomposition", "Break complex objectives into atomic tasks")
            .with_tags(vec!["planning".into(), "decomposition".into(), "breakdown".into(), "tasks".into()])
    )
    .with_skill(
        SkillDescriptor::new("dependency_analysis", "Dependency Analysis", "Identify task dependencies and critical paths")
            .with_tags(vec!["dependencies".into(), "critical_path".into(), "ordering".into(), "sequencing".into()])
    )
    .with_skill(
        SkillDescriptor::new("resource_allocation", "Resource Allocation", "Assign resources to tasks based on skills and availability")
            .with_tags(vec!["resources".into(), "allocation".into(), "assignment".into(), "capacity".into()])
    )
    .with_delegation()
}

/// Create a quality assurance savant blueprint.
///
/// Expert at testing strategies, test case design, bug reporting,
/// and quality metrics.
pub fn qa_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "QA Lead",
        "Ensure software quality through comprehensive testing strategies and systematic validation",
        "You are a QA lead with expertise in test strategy design, automated testing, manual testing, \
         and quality metrics. You design test cases that cover edge cases, integration points, and \
         regression scenarios. You report bugs clearly and verify fixes thoroughly.",
        llm,
        SavantDomain::QualityAssurance,
    )
    .with_skill(
        SkillDescriptor::new("test_design", "Test Design", "Create comprehensive test plans and test cases")
            .with_tags(vec!["testing".into(), "test_cases".into(), "QA".into(), "validation".into()])
    )
    .with_skill(
        SkillDescriptor::new("bug_analysis", "Bug Analysis", "Identify, reproduce, and document software defects")
            .with_tags(vec!["bugs".into(), "defects".into(), "reproduction".into(), "reporting".into()])
    )
    .with_tools(vec!["FileReadTool".into()])
}

/// Create a security savant blueprint.
///
/// Expert at security analysis, vulnerability assessment, and
/// secure coding practices.
pub fn security_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Security Architect",
        "Identify security vulnerabilities and design robust security measures for software systems",
        "You are a security architect with expertise in threat modeling, vulnerability assessment, \
         secure coding practices, and compliance frameworks. You identify OWASP Top 10 vulnerabilities, \
         review authentication/authorization flows, and recommend mitigations.",
        llm,
        SavantDomain::Security,
    )
    .with_skill(
        SkillDescriptor::new("threat_modeling", "Threat Modeling", "Identify and categorize potential security threats")
            .with_tags(vec!["security".into(), "threats".into(), "modeling".into(), "risk".into()])
    )
    .with_skill(
        SkillDescriptor::new("vulnerability_assessment", "Vulnerability Assessment", "Assess code and systems for security vulnerabilities")
            .with_tags(vec!["vulnerability".into(), "assessment".into(), "OWASP".into(), "audit".into()])
    )
    .with_skill(
        SkillDescriptor::new("secure_coding", "Secure Coding Review", "Review code for security best practices")
            .with_tags(vec!["secure".into(), "coding".into(), "review".into(), "best_practices".into()])
    )
    .with_tools(vec!["FileReadTool".into()])
}

/// Create a DevOps savant blueprint.
///
/// Expert at deployment, CI/CD pipelines, containerization, infrastructure
/// as code, monitoring, and cloud platform management.
pub fn devops_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Senior DevOps Engineer",
        "Design, automate, and maintain deployment pipelines, infrastructure, and monitoring systems",
        "You are a senior DevOps engineer with deep expertise in CI/CD, containerization (Docker, \
         Kubernetes), infrastructure as code (Terraform, Pulumi), cloud platforms (AWS, GCP, Azure), \
         and observability (Prometheus, Grafana, OpenTelemetry). You automate everything, ensure \
         reliability through SRE practices, and optimize for cost and performance.",
        llm,
        SavantDomain::DevOps,
    )
    .with_skill(
        SkillDescriptor::new("ci_cd_pipelines", "CI/CD Pipeline Design", "Design and maintain continuous integration and deployment pipelines")
            .with_tags(vec!["ci/cd".into(), "pipeline".into(), "automation".into(), "deploy".into(), "build".into()])
    )
    .with_skill(
        SkillDescriptor::new("containerization", "Containerization", "Build and manage Docker containers and Kubernetes orchestration")
            .with_tags(vec!["docker".into(), "kubernetes".into(), "container".into(), "k8s".into(), "orchestration".into()])
    )
    .with_skill(
        SkillDescriptor::new("infrastructure_as_code", "Infrastructure as Code", "Define and provision infrastructure using code-based tools")
            .with_tags(vec!["terraform".into(), "infrastructure".into(), "cloud".into(), "provisioning".into(), "iac".into()])
    )
    .with_skill(
        SkillDescriptor::new("monitoring_observability", "Monitoring & Observability", "Set up monitoring, alerting, and observability systems")
            .with_tags(vec!["monitoring".into(), "logging".into(), "alerting".into(), "observability".into(), "metrics".into()])
    )
    .with_tools(vec![
        "FileReadTool".into(),
        "FileWriterTool".into(),
        "DirectoryReadTool".into(),
    ])
    .with_delegation()
}

/// Create a design savant blueprint.
///
/// Expert at UX/UI design, design systems, accessibility, prototyping,
/// and user research synthesis.
pub fn design_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Senior UX/UI Designer",
        "Design intuitive, accessible, and visually compelling user interfaces and experiences",
        "You are a senior UX/UI designer with expertise in user-centered design, design systems, \
         accessibility (WCAG), information architecture, and visual design. You create wireframes, \
         prototypes, and design specifications. You synthesize user research into actionable design \
         decisions and maintain consistent design language across products.",
        llm,
        SavantDomain::Design,
    )
    .with_skill(
        SkillDescriptor::new("ux_research_synthesis", "UX Research Synthesis", "Analyze user research data and extract design insights")
            .with_tags(vec!["ux".into(), "research".into(), "user".into(), "personas".into(), "journey".into()])
    )
    .with_skill(
        SkillDescriptor::new("ui_design", "UI Design", "Create visual designs, layouts, and component specifications")
            .with_tags(vec!["ui".into(), "design".into(), "layout".into(), "visual".into(), "components".into()])
    )
    .with_skill(
        SkillDescriptor::new("design_systems", "Design Systems", "Build and maintain consistent design systems and pattern libraries")
            .with_tags(vec!["design_system".into(), "patterns".into(), "tokens".into(), "consistency".into(), "library".into()])
    )
    .with_skill(
        SkillDescriptor::new("accessibility_audit", "Accessibility Audit", "Evaluate and improve designs for accessibility compliance")
            .with_tags(vec!["accessibility".into(), "wcag".into(), "a11y".into(), "inclusive".into(), "aria".into()])
    )
    .with_tools(vec!["FileReadTool".into()])
}

// ---------------------------------------------------------------------------
// Chess savant blueprints — ChessThinkTank agents
// ---------------------------------------------------------------------------

/// Create a chess strategist savant blueprint (the manager agent).
///
/// Expert at high-level chess strategy: opening selection, pawn structure
/// evaluation, long-term planning, and position assessment. Manages the
/// ChessThinkTank crew, delegates to specialists, and makes final move
/// decisions.
pub fn chess_strategist_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Chess Strategist",
        "Analyze chess positions holistically and select the best strategic plan by coordinating specialist agents",
        "You are a grandmaster-level chess strategist who thinks in terms of plans, not just moves. \
         You evaluate pawn structures, piece activity, king safety, and strategic themes. You query \
         the opening book via neo4j_query, find similar positions via ladybug_similarity, and delegate \
         tactical verification to the Tactician. You explain your reasoning as a chain of strategic \
         concepts: space advantage, weak squares, piece coordination, pawn majorities.",
        llm,
        SavantDomain::Chess,
    )
    .with_skill(
        SkillDescriptor::new("position_evaluation", "Position Evaluation", "Assess chess positions for strategic features and imbalances")
            .with_tags(vec!["chess".into(), "evaluation".into(), "strategy".into(), "position".into(), "assessment".into()])
            .with_tools(vec!["chess_evaluate".into(), "neo4j_query".into(), "ladybug_similarity".into()])
            .with_proficiency(0.95)
    )
    .with_skill(
        SkillDescriptor::new("opening_selection", "Opening Selection", "Choose and navigate chess openings based on knowledge graph")
            .with_tags(vec!["chess".into(), "opening".into(), "eco".into(), "repertoire".into()])
            .with_tools(vec!["neo4j_query".into()])
            .with_proficiency(0.9)
    )
    .with_skill(
        SkillDescriptor::new("plan_formation", "Plan Formation", "Formulate long-term strategic plans based on position features")
            .with_tags(vec!["chess".into(), "plan".into(), "strategy".into(), "theme".into()])
            .with_proficiency(0.9)
    )
    .with_tools(vec![
        "chess_evaluate".into(),
        "chess_legal_moves".into(),
        "neo4j_query".into(),
        "ladybug_similarity".into(),
        "chess_whatif".into(),
    ])
    .with_delegation()
}

/// Create a chess tactician savant blueprint.
///
/// Expert at calculating forcing sequences: checks, captures, threats.
/// Verifies candidate moves for tactical soundness using the chess engine.
pub fn chess_tactician_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Chess Tactician",
        "Calculate forcing sequences and verify tactical soundness of candidate moves",
        "You are a tactical calculation specialist. You find combinations, sacrifices, \
         forks, pins, skewers, discovered attacks, and mating patterns. When given candidate \
         moves from the Strategist, you verify them by calculating the critical forcing lines \
         using the chess engine. You report whether a move is tactically sound, and flag any \
         tactical opportunities or dangers the Strategist may have missed.",
        llm,
        SavantDomain::Chess,
    )
    .with_skill(
        SkillDescriptor::new("tactical_calculation", "Tactical Calculation", "Calculate forcing sequences: checks, captures, threats")
            .with_tags(vec!["chess".into(), "tactics".into(), "calculation".into(), "combination".into(), "sacrifice".into()])
            .with_tools(vec!["chess_evaluate".into(), "chess_legal_moves".into()])
            .with_proficiency(0.95)
    )
    .with_skill(
        SkillDescriptor::new("move_verification", "Move Verification", "Verify candidate moves for tactical correctness")
            .with_tags(vec!["chess".into(), "verification".into(), "blunder_check".into()])
            .with_tools(vec!["chess_evaluate".into()])
            .with_proficiency(0.9)
    )
    .with_tools(vec![
        "chess_evaluate".into(),
        "chess_legal_moves".into(),
    ])
}

/// Create a chess endgame specialist savant blueprint.
///
/// Spawned when piece_count < 10. Expert at endgame theory, tablebase
/// knowledge, pawn promotion, and king activity.
pub fn chess_endgame_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Endgame Specialist",
        "Apply endgame theory and tablebase knowledge to convert advantages or hold draws",
        "You are an endgame specialist with encyclopedic knowledge of endgame theory: Lucena \
         and Philidor positions, opposition, triangulation, corresponding squares, zugzwang, \
         and all fundamental endgame types (KR vs K, KP vs K, KBN vs K, rook endgames). \
         You know that in endgames, king activity and passed pawns are paramount. You query \
         the knowledge graph for endgame patterns and similar positions.",
        llm,
        SavantDomain::Chess,
    )
    .with_skill(
        SkillDescriptor::new("endgame_theory", "Endgame Theory", "Apply theoretical endgame knowledge and tablebase results")
            .with_tags(vec!["chess".into(), "endgame".into(), "tablebase".into(), "technique".into()])
            .with_tools(vec!["chess_evaluate".into(), "neo4j_query".into()])
            .with_proficiency(0.9)
    )
    .with_skill(
        SkillDescriptor::new("pawn_endgame", "Pawn Endgame Analysis", "Evaluate pawn structures and promotion races in endgames")
            .with_tags(vec!["chess".into(), "pawn".into(), "promotion".into(), "opposition".into()])
            .with_proficiency(0.85)
    )
    .with_tools(vec![
        "chess_evaluate".into(),
        "chess_legal_moves".into(),
        "neo4j_query".into(),
        "ladybug_similarity".into(),
    ])
}

/// Create a chess psychologist savant blueprint.
///
/// Models opponent behavior using game history. Analyzes tendencies,
/// preferred structures, time management patterns, and blunder
/// likelihood.
pub fn chess_psychologist_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Chess Psychologist",
        "Model opponent behavior and recommend practical decisions based on opponent tendencies",
        "You are an opponent modeling specialist. You analyze the opponent's game history, \
         preferred openings, time management, and error patterns. In positions where multiple \
         plans are equally good objectively, you recommend the one that maximizes practical \
         winning chances against this specific opponent. You consider: does the opponent handle \
         sharp positions well? Do they blunder under time pressure? Do they avoid certain \
         structures?",
        llm,
        SavantDomain::Chess,
    )
    .with_skill(
        SkillDescriptor::new("opponent_modeling", "Opponent Modeling", "Analyze opponent game history and behavioral patterns")
            .with_tags(vec!["chess".into(), "opponent".into(), "psychology".into(), "modeling".into(), "history".into()])
            .with_tools(vec!["neo4j_query".into()])
            .with_proficiency(0.8)
    )
    .with_skill(
        SkillDescriptor::new("practical_play", "Practical Decision Making", "Choose moves that maximize practical winning chances")
            .with_tags(vec!["chess".into(), "practical".into(), "winning_chances".into()])
            .with_proficiency(0.8)
    )
    .with_tools(vec!["neo4j_query".into()])
}

/// Create a chess inner critic savant blueprint.
///
/// Devil's advocate agent that tries to refute proposed moves by finding
/// counterplay, defensive resources, and hidden dangers.
pub fn chess_critic_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Inner Critic",
        "Challenge proposed moves by finding refutations, counterplay, and hidden dangers",
        "You are the devil's advocate in the ChessThinkTank. Your role is to try to refute \
         every proposed move. For each candidate, you search for opponent's best responses, \
         defensive resources, counterattacking possibilities, and tactical traps. You rate \
         your confidence in the refutation. If you cannot find a refutation, the move is \
         likely good. You prevent the team from playing overconfident moves.",
        llm,
        SavantDomain::Chess,
    )
    .with_skill(
        SkillDescriptor::new("refutation_search", "Refutation Search", "Find refutations and counterplay against proposed moves")
            .with_tags(vec!["chess".into(), "refutation".into(), "counterplay".into(), "defense".into()])
            .with_tools(vec!["chess_evaluate".into(), "chess_legal_moves".into()])
            .with_proficiency(0.85)
    )
    .with_skill(
        SkillDescriptor::new("danger_detection", "Danger Detection", "Identify hidden tactical and positional dangers")
            .with_tags(vec!["chess".into(), "danger".into(), "trap".into(), "threat".into()])
            .with_tools(vec!["chess_evaluate".into()])
            .with_proficiency(0.85)
    )
    .with_tools(vec![
        "chess_evaluate".into(),
        "chess_legal_moves".into(),
    ])
}

/// Create a chess advocatus diaboli savant blueprint.
///
/// Full opponent perspective simulator. Unlike the Inner Critic (which looks
/// for refutations), the Advocatus Diaboli role-plays as the opponent: it
/// formulates counterplans, identifies the opponent's best strategic ideas,
/// and stress-tests each candidate move by exploring 32-move what-if
/// branches from the opponent's point of view. It answers: "If I were my
/// opponent, what would I WANT to do here — and does this move let me do it?"
pub fn chess_advocatus_diaboli_savant(llm: &str) -> AgentBlueprint {
    AgentBlueprint::new(
        "Advocatus Diaboli",
        "Simulate the opponent's perspective: formulate their ideal plans, find counterplay, \
         and stress-test candidate moves through opponent-POV what-if branching",
        "You are the Advocatus Diaboli — the Devil's Advocate who fully inhabits the opponent's \
         mind. For every position, you switch sides and ask: 'What is MY best plan as the \
         opponent? What do I WANT to achieve? Which squares am I targeting? Which pieces are \
         poorly placed from my (opponent's) perspective?' You use chess_whatif to generate \
         32-move branches FROM THE OPPONENT'S REPLY, exploring the opponent's best continuations. \
         You combine Psychologist data (opponent tendencies) with Tactician-level calculation. \
         Your output is an adversarial report: for each candidate move, you provide the opponent's \
         best response, their resulting plan, the evaluation swing, and a 'danger score' (0-10). \
         A high danger score means the candidate move walks into the opponent's strengths. \
         You force the team to confront uncomfortable truths about the position.",
        llm,
        SavantDomain::Chess,
    )
    .with_skill(
        SkillDescriptor::new("opponent_simulation", "Opponent Simulation", "Role-play as the opponent to find their best plans and counterplay")
            .with_tags(vec!["chess".into(), "opponent".into(), "simulation".into(), "adversarial".into(), "counterplan".into()])
            .with_tools(vec!["chess_evaluate".into(), "chess_whatif".into(), "chess_legal_moves".into()])
            .with_proficiency(0.9)
    )
    .with_skill(
        SkillDescriptor::new("danger_scoring", "Danger Scoring", "Rate how dangerous each candidate move is from the opponent's perspective")
            .with_tags(vec!["chess".into(), "danger".into(), "risk".into(), "scoring".into(), "adversarial".into()])
            .with_tools(vec!["chess_evaluate".into()])
            .with_proficiency(0.85)
    )
    .with_skill(
        SkillDescriptor::new("counterplan_generation", "Counterplan Generation", "Generate concrete opponent counterplans using what-if branching")
            .with_tags(vec!["chess".into(), "counterplan".into(), "whatif".into(), "branching".into()])
            .with_tools(vec!["chess_whatif".into(), "neo4j_query".into()])
            .with_proficiency(0.85)
    )
    .with_tools(vec![
        "chess_evaluate".into(),
        "chess_legal_moves".into(),
        "chess_whatif".into(),
        "neo4j_query".into(),
    ])
}

/// Get all chess savant blueprints (ChessThinkTank crew).
///
/// Returns the six specialist agents that form the hierarchical chess crew:
/// Strategist (manager), Tactician, Endgame Specialist, Psychologist,
/// Inner Critic, and Advocatus Diaboli (opponent perspective simulator).
pub fn chess_think_tank(llm: &str) -> Vec<AgentBlueprint> {
    vec![
        chess_strategist_savant(llm),
        chess_tactician_savant(llm),
        chess_endgame_savant(llm),
        chess_psychologist_savant(llm),
        chess_critic_savant(llm),
        chess_advocatus_diaboli_savant(llm),
    ]
}

/// Get all available savant blueprints.
///
/// Returns one blueprint for each domain, all using the specified LLM.
pub fn all_savants(llm: &str) -> Vec<AgentBlueprint> {
    vec![
        research_savant(llm),
        engineering_savant(llm),
        data_analysis_savant(llm),
        content_creation_savant(llm),
        planning_savant(llm),
        qa_savant(llm),
        security_savant(llm),
        devops_savant(llm),
        design_savant(llm),
        chess_strategist_savant(llm),
    ]
}

/// Get a savant blueprint for a specific domain.
pub fn savant_for_domain(domain: SavantDomain, llm: &str) -> AgentBlueprint {
    match domain {
        SavantDomain::Research => research_savant(llm),
        SavantDomain::Engineering => engineering_savant(llm),
        SavantDomain::DataAnalysis => data_analysis_savant(llm),
        SavantDomain::ContentCreation => content_creation_savant(llm),
        SavantDomain::Planning => planning_savant(llm),
        SavantDomain::QualityAssurance => qa_savant(llm),
        SavantDomain::Security => security_savant(llm),
        SavantDomain::DevOps => devops_savant(llm),
        SavantDomain::Design => design_savant(llm),
        SavantDomain::Chess => chess_strategist_savant(llm),
        SavantDomain::General => planning_savant(llm),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_savant() {
        let bp = research_savant("openai/gpt-4o");
        assert_eq!(bp.domain, SavantDomain::Research);
        assert!(!bp.skills.is_empty());
        assert!(bp.allow_delegation);
        assert!(!bp.tools.is_empty());
    }

    #[test]
    fn test_engineering_savant() {
        let bp = engineering_savant("anthropic/claude-3-5-sonnet-latest");
        assert_eq!(bp.domain, SavantDomain::Engineering);
        assert!(bp.skills.len() >= 3);
        assert!(bp.tools.contains(&"FileReadTool".to_string()));
    }

    #[test]
    fn test_all_savants() {
        let savants = all_savants("openai/gpt-4o-mini");
        assert_eq!(savants.len(), 10);
        let domains: Vec<_> = savants.iter().map(|s| s.domain).collect();
        assert!(domains.contains(&SavantDomain::Research));
        assert!(domains.contains(&SavantDomain::Engineering));
        assert!(domains.contains(&SavantDomain::Security));
        assert!(domains.contains(&SavantDomain::DevOps));
        assert!(domains.contains(&SavantDomain::Design));
        assert!(domains.contains(&SavantDomain::Chess));
    }

    #[test]
    fn test_chess_think_tank() {
        let agents = chess_think_tank("anthropic/claude-3-5-sonnet-latest");
        assert_eq!(agents.len(), 6);
        assert_eq!(agents[0].role, "Chess Strategist");
        assert_eq!(agents[1].role, "Chess Tactician");
        assert_eq!(agents[2].role, "Endgame Specialist");
        assert_eq!(agents[3].role, "Chess Psychologist");
        assert_eq!(agents[4].role, "Inner Critic");
        assert_eq!(agents[5].role, "Advocatus Diaboli");

        // All should be Chess domain
        for agent in &agents {
            assert_eq!(agent.domain, SavantDomain::Chess);
        }

        // Strategist should have delegation enabled
        assert!(agents[0].allow_delegation);
    }

    #[test]
    fn test_chess_advocatus_diaboli() {
        let bp = chess_advocatus_diaboli_savant("openai/gpt-4o");
        assert_eq!(bp.domain, SavantDomain::Chess);
        assert_eq!(bp.role, "Advocatus Diaboli");
        assert!(bp.tools.contains(&"chess_whatif".to_string()));
        assert!(bp.tools.contains(&"chess_evaluate".to_string()));
        assert!(bp.tools.contains(&"neo4j_query".to_string()));
        assert_eq!(bp.skills.len(), 3);
        // Advocatus Diaboli does NOT have delegation (it's a specialist, not a manager)
        assert!(!bp.allow_delegation);
    }

    #[test]
    fn test_chess_strategist_tools() {
        let bp = chess_strategist_savant("openai/gpt-4o");
        assert!(bp.tools.contains(&"chess_evaluate".to_string()));
        assert!(bp.tools.contains(&"neo4j_query".to_string()));
        assert!(bp.tools.contains(&"ladybug_similarity".to_string()));
        assert!(!bp.skills.is_empty());
    }

    #[test]
    fn test_chess_savant_for_domain() {
        let bp = savant_for_domain(SavantDomain::Chess, "openai/gpt-4o");
        assert_eq!(bp.domain, SavantDomain::Chess);
        assert_eq!(bp.role, "Chess Strategist");
    }

    #[test]
    fn test_devops_savant() {
        let bp = devops_savant("openai/gpt-4o");
        assert_eq!(bp.domain, SavantDomain::DevOps);
        assert!(bp.skills.len() >= 4);
        assert!(bp.allow_delegation);
        assert!(!bp.tools.is_empty());
    }

    #[test]
    fn test_design_savant() {
        let bp = design_savant("anthropic/claude-3-5-sonnet-latest");
        assert_eq!(bp.domain, SavantDomain::Design);
        assert!(bp.skills.len() >= 4);
        assert!(bp.tools.contains(&"FileReadTool".to_string()));
    }

    #[test]
    fn test_savant_for_domain() {
        let bp = savant_for_domain(SavantDomain::Security, "xai/grok-3");
        assert_eq!(bp.domain, SavantDomain::Security);
        assert_eq!(bp.llm, "xai/grok-3");
    }

    #[test]
    fn test_savant_skills_have_tags() {
        let bp = research_savant("openai/gpt-4o");
        for skill in &bp.skills {
            assert!(!skill.tags.is_empty(), "Skill {} should have tags", skill.id);
        }
    }
}
