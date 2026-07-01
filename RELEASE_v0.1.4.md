# OmniOwn v0.1.4 Release Notes

**Release Date:** 2026-06-25

## Overview

v0.1.4 introduces a **two-stage AI search pipeline** with intelligent query analysis and strategy selection. Enhanced search accuracy through LLM-powered query understanding, combined with structured validation and document library context awareness.

## Key Features

### 🤖 Two-Stage AI Search Pipeline
- **Stage 1 (Query Analysis):** LLM-powered preprocessing that rewrites queries, extracts keywords, and detects intent, category preferences, filetype filters, and time-range constraints
- **Stage 2 (Strategy Selection):** Intelligent strategy selection with JSON Schema validation via Zod, ensuring only valid strategies are executed

### 🔍 Enhanced Search Context
- **Document Stats Cache:** Total document count and category distribution injected into the v2 prompt system
- **60-second TTL Cache:** Automatically invalidated on import and watch events
- **Tiered Result Merging:** FTS results prioritized; non-FTS results capped at 5 when FTS exists to reduce noise

### 🌐 Improved Internationalization
- All system prompts converted to English instructions with mixed CN/EN few-shot examples
- Better JSON compliance for LLM outputs
- Modularized prompt architecture for easier maintenance and reuse

## Changes

### Added
- **Query Analysis Prompt Module** (`server/src/prompts/query-analysis.prompt.ts`)
  - Two-phase LLM query understanding: rewrite + intent/category/filetype/time-range detection
  - Structured JSON output for programmatic processing
  
- **Zod JSON Schema Validation**
  - Strategy selection output validated against `StrategyResponseSchema`
  - Enum-checked strategy names, minimum 1 strategy requirement
  - Descriptive error messages on validation failure

- **Document Library Context**
  - `getDocumentStats()` service for gathering library metadata
  - Automatic cache invalidation on document changes
  - Context injection into v2 prompt as `[Document Library Info]` block

### Fixed
- **v2 Context Never Injected:** `getDocumentStats()` now properly called in `selectStrategies` when variant is 'v2'
- **Search context availability verified and properly passed through the pipeline**

### Improved
- **Prompt Modularization:** Prompts extracted from `ai.service.ts` into dedicated `prompts/` module
  - `search-strategy.prompt.ts` with v1/v2 variants
  - 6 few-shot examples per strategy
  - Context injection and intelligent fallback
  - `index.ts` barrel exports for clean imports

- **Result Quality:** Tiered result merging reduces noise while preserving browsing capabilities

## Technical Improvements

- ✅ Replaced bare `as StrategyCall[]` type assertions with proper Zod validation
- ✅ Cache invalidation integrated with import and watch workflows
- ✅ Prompt system isolation enables easier testing and maintenance
- ✅ Better error messages for debugging search issues

## Breaking Changes

None — This release maintains backward compatibility with v0.1.3.

## Migration Guide

No action required. Simply upgrade to v0.1.4 and enjoy improved search accuracy.

## Performance

- Query analysis adds ~200-500ms depending on LLM endpoint
- Document stats cache reduces overhead on repeated queries
- Result filtering improves UI responsiveness with large document sets

## Related Documentation

- [Architecture Overview](docs/architecture.md) — System design and component interactions
- [Search Strategy Guide](docs/cli.md) — CLI usage and search capabilities
- [Development Guide](docs/development.md) — Setup and contribution workflow

## Known Limitations

- LLM query analysis depends on API availability
- Category detection accuracy varies by document naming conventions
- Time-range extraction requires ISO 8601 or common date formats

## Contributors

Thanks to all contributors who helped shape v0.1.4! 🙏

---

**Download:** [Release Assets on GitHub](https://github.com/yourusername/omniown/releases/tag/v0.1.4)

**Report Issues:** [GitHub Issues](https://github.com/yourusername/omniown/issues)
