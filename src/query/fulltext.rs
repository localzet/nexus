//! Full-Text Search & Text Indexing - Полнотекстовый поиск
/// Inverted index, tokenization, relevance scoring
use std::collections::{HashMap, HashSet};

/// Token from text
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub term: String,
    pub position: u32,
    pub frequency: u32,
}

impl Token {
    pub fn new(term: String, position: u32) -> Self {
        Self {
            term,
            position,
            frequency: 1,
        }
    }
}

/// Document metadata for search
#[derive(Debug, Clone)]
pub struct Document {
    pub doc_id: u32,
    pub title: String,
    pub content: String,
    pub length: u32,
    pub tokens: Vec<Token>,
}

impl Document {
    pub fn new(doc_id: u32, title: String, content: String) -> Self {
        let length = content.split_whitespace().count() as u32;
        Self {
            doc_id,
            title,
            content,
            length,
            tokens: Vec::new(),
        }
    }

    pub fn set_tokens(&mut self, tokens: Vec<Token>) {
        self.tokens = tokens;
    }
}

/// Inverted index posting
#[derive(Debug, Clone)]
pub struct Posting {
    pub doc_id: u32,
    pub positions: Vec<u32>,
    pub term_frequency: u32,
}

impl Posting {
    pub fn new(doc_id: u32) -> Self {
        Self {
            doc_id,
            positions: Vec::new(),
            term_frequency: 0,
        }
    }

    pub fn add_position(&mut self, position: u32) {
        self.positions.push(position);
        self.term_frequency += 1;
    }
}

/// Inverted index for fast text search
#[derive(Debug, Clone)]
pub struct InvertedIndex {
    index: HashMap<String, Vec<Posting>>,
    document_count: u32,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            document_count: 0,
        }
    }

    /// Add document to index
    pub fn add_document(&mut self, doc: &Document) {
        for token in &doc.tokens {
            self.index
                .entry(token.term.clone())
                .or_insert_with(Vec::new)
                .push(Posting::new(doc.doc_id));
        }
        self.document_count += 1;
    }

    /// Search for term
    pub fn search_term(&self, term: &str) -> Option<&Vec<Posting>> {
        self.index.get(term)
    }

    /// Search for multiple terms (AND query)
    pub fn search_and(&self, terms: &[&str]) -> Vec<u32> {
        if terms.is_empty() {
            return Vec::new();
        }

        let mut result: Option<Vec<u32>> = None;

        for term in terms {
            if let Some(postings) = self.search_term(term) {
                let doc_ids: Vec<u32> = postings.iter().map(|p| p.doc_id).collect();

                result = match result {
                    None => Some(doc_ids),
                    Some(prev) => {
                        let intersection: HashSet<u32> = prev.into_iter().collect();
                        Some(doc_ids.into_iter().filter(|id| intersection.contains(id)).collect())
                    }
                };
            } else {
                return Vec::new();
            }
        }

        result.unwrap_or_default()
    }

    /// Search for multiple terms (OR query)
    pub fn search_or(&self, terms: &[&str]) -> Vec<u32> {
        let mut result = HashSet::new();

        for term in terms {
            if let Some(postings) = self.search_term(term) {
                for posting in postings {
                    result.insert(posting.doc_id);
                }
            }
        }

        let mut doc_ids: Vec<u32> = result.into_iter().collect();
        doc_ids.sort();
        doc_ids
    }

    /// Get statistics
    pub fn get_term_count(&self) -> usize {
        self.index.len()
    }

    /// Get document count
    pub fn get_document_count(&self) -> u32 {
        self.document_count
    }
}

/// Simple tokenizer
pub struct Tokenizer;

impl Tokenizer {
    /// Tokenize text with position
    pub fn tokenize(text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut position = 0u32;

        for word in text.split_whitespace() {
            let term = word.to_lowercase();
            // Filter out common punctuation
            let term = term.trim_matches(|c| !char::is_alphanumeric(c));
            
            if !term.is_empty() && term.len() > 1 {
                tokens.push(Token::new(term.to_string(), position));
                position += 1;
            }
        }

        tokens
    }

    /// Remove stop words
    pub fn remove_stopwords(tokens: &[Token]) -> Vec<Token> {
        let stopwords = vec![
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "is", "are"
        ];
        
        tokens
            .iter()
            .filter(|token| !stopwords.contains(&token.term.as_str()))
            .cloned()
            .collect()
    }

    /// Apply stemming (simplified Porter stemmer)
    pub fn stem_token(term: &str) -> String {
        // Simplified stemming rules
        let term = term.trim_end_matches('s');
        let term = term.trim_end_matches("ed");
        let term = term.trim_end_matches("ing");
        term.to_string()
    }

    /// Apply stemming to tokens
    pub fn stem_tokens(tokens: &[Token]) -> Vec<Token> {
        tokens
            .iter()
            .map(|token| Token {
                term: Self::stem_token(&token.term),
                ..token.clone()
            })
            .collect()
    }
}

/// BM25 relevance scorer
pub struct BM25Scorer {
    k1: f64, // Term saturation parameter
    b: f64,  // Field length normalization
}

impl BM25Scorer {
    pub fn new(k1: f64, b: f64) -> Self {
        Self { k1, b }
    }

    /// Calculate BM25 score
    pub fn score(
        &self,
        term_freq: u32,
        doc_length: u32,
        avg_doc_length: f64,
        idf: f64,
    ) -> f64 {
        let numerator = (self.k1 + 1.0) * (term_freq as f64);
        let length_norm = 1.0 - self.b + self.b * ((doc_length as f64) / avg_doc_length);
        let denominator = (term_freq as f64) + self.k1 * length_norm;

        (numerator / denominator) * idf
    }

    /// Calculate IDF (Inverse Document Frequency)
    pub fn calculate_idf(doc_count: u32, docs_with_term: u32) -> f64 {
        if docs_with_term == 0 {
            0.0
        } else {
            ((doc_count as f64 - docs_with_term as f64 + 0.5) 
                / (docs_with_term as f64 + 0.5) + 1.0).ln()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let token = Token::new("hello".to_string(), 0);
        assert_eq!(token.term, "hello");
        assert_eq!(token.position, 0);
        assert_eq!(token.frequency, 1);
    }

    #[test]
    fn test_posting_creation() {
        let posting = Posting::new(1);
        assert_eq!(posting.doc_id, 1);
        assert_eq!(posting.term_frequency, 0);
    }

    #[test]
    fn test_posting_add_position() {
        let mut posting = Posting::new(1);
        posting.add_position(0);
        posting.add_position(5);
        assert_eq!(posting.positions.len(), 2);
        assert_eq!(posting.term_frequency, 2);
    }

    #[test]
    fn test_document_creation() {
        let doc = Document::new(1, "Title".to_string(), "The quick brown fox".to_string());
        assert_eq!(doc.doc_id, 1);
        assert_eq!(doc.title, "Title");
        assert_eq!(doc.length, 4);
    }

    #[test]
    fn test_inverted_index_creation() {
        let index = InvertedIndex::new();
        assert_eq!(index.get_term_count(), 0);
        assert_eq!(index.get_document_count(), 0);
    }

    #[test]
    fn test_inverted_index_add_document() {
        let mut index = InvertedIndex::new();
        let mut doc = Document::new(1, "Test".to_string(), "hello world".to_string());
        doc.set_tokens(vec![
            Token::new("hello".to_string(), 0),
            Token::new("world".to_string(), 1),
        ]);
        
        index.add_document(&doc);
        assert_eq!(index.get_document_count(), 1);
        assert_eq!(index.get_term_count(), 2);
    }

    #[test]
    fn test_inverted_index_search_term() {
        let mut index = InvertedIndex::new();
        let mut doc = Document::new(1, "Test".to_string(), "hello world".to_string());
        doc.set_tokens(vec![Token::new("hello".to_string(), 0)]);
        
        index.add_document(&doc);
        assert!(index.search_term("hello").is_some());
        assert!(index.search_term("goodbye").is_none());
    }

    #[test]
    fn test_inverted_index_search_and() {
        let mut index = InvertedIndex::new();
        let mut doc1 = Document::new(1, "Test1".to_string(), "hello world".to_string());
        doc1.set_tokens(vec![
            Token::new("hello".to_string(), 0),
            Token::new("world".to_string(), 1),
        ]);
        
        let mut doc2 = Document::new(2, "Test2".to_string(), "hello there".to_string());
        doc2.set_tokens(vec![Token::new("hello".to_string(), 0)]);
        
        index.add_document(&doc1);
        index.add_document(&doc2);
        
        let results = index.search_and(&["hello", "world"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 1);
    }

    #[test]
    fn test_inverted_index_search_or() {
        let mut index = InvertedIndex::new();
        let mut doc1 = Document::new(1, "Test1".to_string(), "hello".to_string());
        doc1.set_tokens(vec![Token::new("hello".to_string(), 0)]);
        
        let mut doc2 = Document::new(2, "Test2".to_string(), "world".to_string());
        doc2.set_tokens(vec![Token::new("world".to_string(), 0)]);
        
        index.add_document(&doc1);
        index.add_document(&doc2);
        
        let results = index.search_or(&["hello", "world"]);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_tokenizer_tokenize() {
        let text = "Hello world example text";
        let tokens = Tokenizer::tokenize(text);
        assert!(tokens.len() > 0);
        assert_eq!(tokens[0].term, "hello");
    }

    #[test]
    fn test_tokenizer_remove_stopwords() {
        let tokens = vec![
            Token::new("hello".to_string(), 0),
            Token::new("the".to_string(), 1),
            Token::new("world".to_string(), 2),
        ];
        
        let filtered = Tokenizer::remove_stopwords(&tokens);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_tokenizer_stem() {
        assert_eq!(Tokenizer::stem_token("running"), "runn");
        assert_eq!(Tokenizer::stem_token("books"), "book");
        assert_eq!(Tokenizer::stem_token("walked"), "walk");
    }

    #[test]
    fn test_bm25_scorer_creation() {
        let scorer = BM25Scorer::new(1.5, 0.75);
        assert_eq!(scorer.k1, 1.5);
        assert_eq!(scorer.b, 0.75);
    }

    #[test]
    fn test_bm25_idf_calculation() {
        let idf = BM25Scorer::calculate_idf(100, 10);
        assert!(idf > 0.0);
    }

    #[test]
    fn test_bm25_score() {
        let scorer = BM25Scorer::new(1.5, 0.75);
        let score = scorer.score(5, 100, 80.0, 2.0);
        assert!(score > 0.0);
    }
}
