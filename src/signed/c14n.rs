//! Canonical `XML` 1.0 and exclusive `XML` canonicalization 1.0.
//!
//! The input is read into a [`Tree`], a read-only view of the document as
//! the `XPath` data model sees it. Line ends are normalized, attribute values
//! carry the `XML` 1.0 attribute-value normalization, character and
//! predefined entity references are resolved, and every element keeps the
//! qualified name it was written with, which canonical output must repeat.
//! A document with a `DOCTYPE` is refused, so no entity or attribute default
//! declared in a `DTD` can make the tree differ from the bytes a schema-driven
//! reader sees.
//!
//! [`canonicalize`] writes a node set of the tree in canonical form. The
//! node sets the signature references produce are the whole document or the
//! subtree of one element, each minus the subtrees of the excluded elements,
//! with or without the comments.

use std::collections::BTreeMap;
use std::rc::Rc;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

/// The namespace bound to the `xml` prefix.
pub(crate) const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// The index of a node in a [`Tree`].
pub(crate) type NodeId = usize;

/// Why a document could not be read into a [`Tree`].
#[derive(Debug, thiserror::Error)]
pub(crate) enum TreeError {
    /// The declared `encoding` is unsupported or the bytes do not decode.
    #[error("the document does not decode: {0}")]
    Encoding(#[source] Box<crate::Error>),

    /// The input is not well-formed `XML`.
    #[error("not well-formed XML: {0}")]
    Malformed(String),

    /// The document carries a `DOCTYPE` declaration.
    #[error("the document carries a DOCTYPE declaration")]
    Doctype,
}

/// A node of a [`Tree`].
#[derive(Debug)]
pub(crate) enum Node {
    /// An element.
    Element(Element),
    /// The character data between two markup items, references resolved.
    Text(String),
    /// A comment.
    Comment(String),
    /// A processing instruction.
    Instruction {
        /// The target.
        target: String,
        /// The data after the target, without the separating white space.
        data: String,
    },
}

impl Node {
    /// The element, when the node is one.
    const fn as_element(&self) -> Option<&Element> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }
}

/// An element of a [`Tree`].
#[derive(Debug)]
pub(crate) struct Element {
    /// The qualified name as written.
    name: String,
    /// The namespace of the element, empty when it has none.
    namespace: String,
    /// The local part of the name.
    local: String,
    /// The namespace declarations of the start tag, the default namespace
    /// under the empty prefix. An empty value undeclares the default.
    declarations: Vec<(String, String)>,
    /// The attributes, without the namespace declarations.
    attributes: Vec<Attribute>,
    /// The child nodes in document order.
    children: Vec<NodeId>,
}

/// An attribute of an [`Element`].
#[derive(Debug)]
pub(crate) struct Attribute {
    /// The qualified name as written.
    name: String,
    /// The namespace of the attribute, empty for an unprefixed one.
    namespace: String,
    /// The local part of the name.
    local: String,
    /// The normalized value.
    value: String,
}

/// A read-only view of a document.
#[derive(Debug)]
pub(crate) struct Tree {
    /// Every node, in document order.
    nodes: Vec<Node>,
    /// The parent element of each node, `None` at the top level.
    parents: Vec<Option<NodeId>>,
    /// The top-level nodes, the document element among the comments and
    /// processing instructions around it.
    top: Vec<NodeId>,
    /// The document element.
    root: NodeId,
}

impl Tree {
    /// Reads `input`, in the encoding its byte order mark or declaration
    /// names.
    pub(crate) fn parse(input: &[u8]) -> Result<Self, TreeError> {
        let utf8 = crate::to_utf8(input).map_err(|error| TreeError::Encoding(Box::new(error)))?;
        let mut builder = Builder::default();
        let mut reader = Reader::from_reader(utf8.as_ref());
        let mut buffer = Vec::new();
        loop {
            let event = reader
                .read_event_into(&mut buffer)
                .map_err(|error| TreeError::Malformed(error.to_string()))?;
            match event {
                Event::Start(start) => {
                    let id = builder.element(&start)?;
                    builder.open.push(id);
                }
                Event::Empty(start) => {
                    builder.element(&start)?;
                }
                Event::End(_) => {
                    builder.open.pop();
                }
                Event::Text(text) => builder.text(&text.xml10_content()),
                Event::CData(data) => builder.text(&data.xml10_content()),
                Event::GeneralRef(reference) => {
                    let character = match reference.resolve_char_ref() {
                        Ok(Some(character)) => character,
                        Ok(None) => predefined_entity(&reference)
                            .ok_or_else(|| TreeError::Malformed(format!("&{};", &*reference)))?,
                        Err(error) => return Err(TreeError::Malformed(error.to_string())),
                    };
                    builder.text(character.encode_utf8(&mut [0; 4]));
                }
                Event::Comment(comment) => {
                    builder.push(Node::Comment(comment.xml10_content().into_owned()));
                }
                Event::PI(instruction) => {
                    let data = normalize_line_ends(
                        instruction
                            .content()
                            .trim_start_matches(['\u{20}', '\t', '\r', '\n']),
                    );
                    builder.push(Node::Instruction {
                        target: instruction.target().to_owned(),
                        data,
                    });
                }
                Event::DocType(_) => return Err(TreeError::Doctype),
                Event::Decl(_) => {}
                Event::Eof => break,
            }
            buffer.clear();
        }
        builder.finish()
    }

    /// The document element.
    pub(crate) const fn root(&self) -> NodeId {
        self.root
    }

    /// The element `id`, when the node is one.
    pub(crate) fn element(&self, id: NodeId) -> Option<&Element> {
        self.nodes.get(id).and_then(Node::as_element)
    }

    /// The parent element of `id`.
    pub(crate) fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.parents.get(id).copied().flatten()
    }

    /// The element `id` and its ancestors, innermost first.
    fn lineage(&self, id: NodeId) -> impl Iterator<Item = &Element> {
        std::iter::successors(Some(id), |&node| self.parent(node))
            .filter_map(|node| self.element(node))
    }

    /// Every element with its id, in document order.
    pub(crate) fn elements(&self) -> impl Iterator<Item = (NodeId, &Element)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(id, node)| node.as_element().map(|element| (id, element)))
    }

    /// The child elements of the element `id` with their ids, in document
    /// order.
    pub(crate) fn child_elements(&self, id: NodeId) -> impl Iterator<Item = (NodeId, &Element)> {
        self.element(id)
            .map_or(&[][..], Element::children)
            .iter()
            .filter_map(|&child| self.element(child).map(|element| (child, element)))
    }

    /// The text children of the element `id`, concatenated.
    pub(crate) fn text(&self, id: NodeId) -> String {
        let mut text = String::new();
        for &child in self.element(id).map_or(&[][..], Element::children) {
            if let Some(Node::Text(part)) = self.nodes.get(child) {
                text.push_str(part);
            }
        }
        text
    }

    /// The namespace `prefix` is bound to at the element `id`, the default
    /// namespace under the empty prefix.
    pub(crate) fn namespace_of(&self, id: NodeId, prefix: &str) -> Option<&str> {
        if prefix == "xml" {
            return Some(XML_NAMESPACE);
        }
        self.lineage(id)
            .find_map(|element| {
                element
                    .declarations
                    .iter()
                    .rev()
                    .find(|(declared, _)| declared == prefix)
            })
            .and_then(|(_, uri)| (!uri.is_empty()).then_some(uri.as_str()))
    }

    /// The namespaces in scope at the element `id`, by prefix, the default
    /// namespace under the empty prefix with an empty value when undeclared.
    fn in_scope(&self, id: NodeId) -> BTreeMap<&str, &str> {
        let lineage: Vec<&Element> = self.lineage(id).collect();
        let mut scope = BTreeMap::new();
        for element in lineage.into_iter().rev() {
            for (prefix, uri) in &element.declarations {
                scope.insert(prefix.as_str(), uri.as_str());
            }
        }
        scope
    }
}

impl Element {
    /// The namespace of the element, empty when it has none.
    pub(crate) fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The local part of the name.
    pub(crate) fn local(&self) -> &str {
        &self.local
    }

    /// The child nodes in document order.
    pub(crate) fn children(&self) -> &[NodeId] {
        &self.children
    }

    /// The value of the unprefixed attribute `local`.
    pub(crate) fn attribute(&self, local: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.namespace.is_empty() && attribute.local == local)
            .map(|attribute| attribute.value.as_str())
    }
}

/// The character of a predefined entity reference.
fn predefined_entity(name: &str) -> Option<char> {
    match name {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "apos" => Some('\''),
        "quot" => Some('"'),
        _ => None,
    }
}

/// `text` with every `\r\n` and lone `\r` turned into `\n`.
fn normalize_line_ends(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// The state of a [`Tree`] under construction.
#[derive(Default)]
struct Builder {
    /// The nodes read so far.
    nodes: Vec<Node>,
    /// The parent of each node read so far.
    parents: Vec<Option<NodeId>>,
    /// The top-level nodes read so far.
    top: Vec<NodeId>,
    /// The elements whose end tag is still to come, innermost last.
    open: Vec<NodeId>,
    /// The document element, once its start tag is read.
    root: Option<NodeId>,
}

impl Builder {
    /// Appends `node` to the innermost open element, or to the top level.
    fn push(&mut self, node: Node) -> NodeId {
        let id = self.nodes.len();
        let parent = self.open.last().copied();
        self.nodes.push(node);
        self.parents.push(parent);
        match parent.and_then(|parent| self.nodes.get_mut(parent)) {
            Some(Node::Element(element)) => element.children.push(id),
            _ => self.top.push(id),
        }
        id
    }

    /// Appends character data to the innermost open element, merging it
    /// into a text node that ends there. Character data outside the
    /// document element is white space, which the data model drops.
    fn text(&mut self, text: &str) {
        let Some(&parent) = self.open.last() else {
            return;
        };
        let last = self
            .nodes
            .get(parent)
            .and_then(Node::as_element)
            .and_then(|element| element.children.last().copied());
        if let Some(last) = last
            && let Some(Node::Text(existing)) = self.nodes.get_mut(last)
        {
            existing.push_str(text);
            return;
        }
        self.push(Node::Text(text.to_owned()));
    }

    /// Reads a start tag into an element and appends it.
    fn element(&mut self, start: &BytesStart<'_>) -> Result<NodeId, TreeError> {
        let mut declarations = Vec::new();
        let mut raw = Vec::new();
        for attribute in start.attributes() {
            let attribute = attribute.map_err(|error| TreeError::Malformed(error.to_string()))?;
            let name = attribute.key.0;
            let value = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|error| TreeError::Malformed(error.to_string()))?
                .into_owned();
            if name == "xmlns" {
                declarations.push((String::new(), value));
            } else if let Some(prefix) = name.strip_prefix("xmlns:") {
                if prefix != "xml" {
                    declarations.push((prefix.to_owned(), value));
                }
            } else {
                raw.push((name.to_owned(), value));
            }
        }
        if self.open.is_empty() {
            if self.root.is_some() {
                return Err(TreeError::Malformed(
                    "more than one document element".to_owned(),
                ));
            }
            self.root = Some(self.nodes.len());
        }
        let name = start.name().0.to_owned();
        let (prefix, local) = split_name(&name);
        let namespace = self.resolve(prefix, &declarations)?;
        let mut attributes = Vec::with_capacity(raw.len());
        for (attribute_name, value) in raw {
            let (attribute_prefix, attribute_local) = split_name(&attribute_name);
            let attribute_namespace = match attribute_prefix {
                Some(_) => self.resolve(attribute_prefix, &declarations)?,
                None => String::new(),
            };
            attributes.push(Attribute {
                local: attribute_local.to_owned(),
                namespace: attribute_namespace,
                name: attribute_name,
                value,
            });
        }
        let local = local.to_owned();
        Ok(self.push(Node::Element(Element {
            name,
            namespace,
            local,
            declarations,
            attributes,
            children: Vec::new(),
        })))
    }

    /// The namespace of `prefix` at a start tag carrying `declarations`, the
    /// default namespace for an unprefixed element name.
    fn resolve(
        &self,
        prefix: Option<&str>,
        declarations: &[(String, String)],
    ) -> Result<String, TreeError> {
        let wanted = match prefix {
            Some("xml") => return Ok(XML_NAMESPACE.to_owned()),
            Some(prefix) => prefix,
            None => "",
        };
        let own = declarations
            .iter()
            .rev()
            .find(|(declared, _)| declared == wanted)
            .map(|(_, uri)| uri.as_str());
        let inherited = || {
            self.open.iter().rev().find_map(|&open| {
                self.nodes
                    .get(open)
                    .and_then(Node::as_element)?
                    .declarations
                    .iter()
                    .rev()
                    .find(|(declared, _)| declared == wanted)
                    .map(|(_, uri)| uri.as_str())
            })
        };
        match own.or_else(inherited) {
            Some(uri) if !uri.is_empty() || wanted.is_empty() => Ok(uri.to_owned()),
            None if wanted.is_empty() => Ok(String::new()),
            _ => Err(TreeError::Malformed(format!(
                "the prefix {wanted} is not declared"
            ))),
        }
    }

    /// The finished tree, once every element is closed.
    fn finish(self) -> Result<Tree, TreeError> {
        let root = self
            .root
            .ok_or_else(|| TreeError::Malformed("no document element".to_owned()))?;
        if !self.open.is_empty() {
            return Err(TreeError::Malformed(
                "the document element is not closed".to_owned(),
            ));
        }
        Ok(Tree {
            nodes: self.nodes,
            parents: self.parents,
            top: self.top,
            root,
        })
    }
}

/// The prefix and the local part of a qualified name.
fn split_name(name: &str) -> (Option<&str>, &str) {
    match name.split_once(':') {
        Some((prefix, local)) => (Some(prefix), local),
        None => (None, name),
    }
}

/// The canonicalization algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Method {
    /// Canonical `XML` 1.0.
    Inclusive,
    /// Exclusive `XML` canonicalization 1.0.
    Exclusive,
}

/// A canonicalization algorithm with its parameters.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Canonicalization<'a> {
    /// The algorithm.
    pub(crate) method: Method,
    /// Whether comments are kept.
    pub(crate) comments: bool,
    /// The `InclusiveNamespaces PrefixList` of the exclusive algorithm, the
    /// default namespace as the empty prefix.
    pub(crate) inclusive_prefixes: &'a [&'a str],
}

/// The node set to canonicalize.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Subset<'a> {
    /// The whole document minus the subtrees of `excluded`.
    Document {
        /// The elements whose subtrees are left out.
        excluded: &'a [NodeId],
    },
    /// The subtree of `apex` minus the subtrees of `excluded`.
    Element {
        /// The element at the top of the subtree.
        apex: NodeId,
        /// The elements whose subtrees are left out.
        excluded: &'a [NodeId],
    },
}

/// Writes `subset` of `tree` in the canonical form of `algorithm`, `UTF-8`
/// encoded.
pub(crate) fn canonicalize(
    tree: &Tree,
    subset: Subset<'_>,
    algorithm: Canonicalization<'_>,
) -> Vec<u8> {
    let (excluded, apex) = match subset {
        Subset::Document { excluded } => (excluded, None),
        Subset::Element { apex, excluded } => (excluded, Some(apex)),
    };
    let mut writer = Writer {
        tree,
        algorithm,
        excluded,
        out: String::new(),
    };
    match apex {
        Some(apex) => {
            let mut ancestor = tree.parent(apex);
            while let Some(node) = ancestor {
                if excluded.contains(&node) {
                    return Vec::new();
                }
                ancestor = tree.parent(node);
            }
            writer.subtree(apex, true);
        }
        None => writer.document(),
    }
    writer.out.into_bytes()
}

/// A step of the iterative walk of a subtree.
enum Step<'t> {
    /// Write the node and, for an element, its start tag and children.
    Enter(NodeId, &'t Node),
    /// Write the end tag of the element with this name.
    Leave(&'t str),
}

/// The namespace context an open element hands to its children.
struct Frame<'t> {
    /// The namespaces in scope at the element.
    in_scope: Rc<BTreeMap<&'t str, &'t str>>,
    /// The namespace declarations rendered on the element and its output
    /// ancestors, for the exclusive algorithm.
    rendered: Rc<BTreeMap<&'t str, &'t str>>,
}

/// The canonical writer of one node set.
struct Writer<'t> {
    /// The document.
    tree: &'t Tree,
    /// The algorithm.
    algorithm: Canonicalization<'t>,
    /// The elements whose subtrees are left out.
    excluded: &'t [NodeId],
    /// The output.
    out: String,
}

impl<'t> Writer<'t> {
    /// Writes the whole document, the top-level comments and processing
    /// instructions separated from the document element by line feeds.
    fn document(&mut self) {
        let tree = self.tree;
        let mut before = true;
        for &id in &tree.top {
            if id == tree.root {
                self.subtree(id, false);
                before = false;
            } else if tree.nodes.get(id).is_some_and(|node| self.writes(id, node)) {
                if !before {
                    self.out.push('\n');
                }
                self.subtree(id, false);
                if before {
                    self.out.push('\n');
                }
            }
        }
    }

    /// Whether the node `node` with the id `id` is written at all.
    fn writes(&self, id: NodeId, node: &Node) -> bool {
        match node {
            Node::Comment(_) => self.algorithm.comments,
            _ => !self.excluded.contains(&id),
        }
    }

    /// Writes the subtree of `apex`, whose ancestors are not output.
    /// `inherit` adds the `xml:` attributes of those ancestors to the apex
    /// under the inclusive algorithm, as a document subset requires.
    fn subtree(&mut self, apex: NodeId, inherit: bool) {
        let tree = self.tree;
        let enter = |id: NodeId| tree.nodes.get(id).map(|node| Step::Enter(id, node));
        let mut steps: Vec<Step<'t>> = enter(apex).into_iter().collect();
        let mut frames: Vec<Frame<'t>> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter(id, node) if !self.writes(id, node) => {}
                Step::Enter(id, Node::Element(element)) => {
                    let frame = self.start_tag(id, element, frames.last(), inherit && id == apex);
                    frames.push(frame);
                    steps.push(Step::Leave(&element.name));
                    steps.extend(
                        element
                            .children
                            .iter()
                            .rev()
                            .filter_map(|&child| enter(child)),
                    );
                }
                Step::Enter(_, Node::Text(text)) => escape_text(&mut self.out, text),
                Step::Enter(_, Node::Comment(comment)) => {
                    self.out.push_str("<!--");
                    self.out.push_str(comment);
                    self.out.push_str("-->");
                }
                Step::Enter(_, Node::Instruction { target, data }) => {
                    self.out.push_str("<?");
                    self.out.push_str(target);
                    if !data.is_empty() {
                        self.out.push(' ');
                        self.out.push_str(data);
                    }
                    self.out.push_str("?>");
                }
                Step::Leave(name) => {
                    self.out.push_str("</");
                    self.out.push_str(name);
                    self.out.push('>');
                    frames.pop();
                }
            }
        }
    }

    /// Writes the start tag of `element` under the context of its output
    /// parent, `None` at the top of the node set, and returns the context it
    /// hands to its children.
    fn start_tag(
        &mut self,
        id: NodeId,
        element: &'t Element,
        parent: Option<&Frame<'t>>,
        inherit: bool,
    ) -> Frame<'t> {
        let in_scope = match parent {
            Some(parent) if element.declarations.is_empty() => Rc::clone(&parent.in_scope),
            Some(parent) => {
                let mut scope = (*parent.in_scope).clone();
                for (prefix, uri) in &element.declarations {
                    scope.insert(prefix.as_str(), uri.as_str());
                }
                Rc::new(scope)
            }
            None => Rc::new(self.tree.in_scope(id)),
        };
        let empty = BTreeMap::new();
        let mut rendered = parent.map_or_else(
            || Rc::new(BTreeMap::new()),
            |frame| Rc::clone(&frame.rendered),
        );
        let declarations: BTreeMap<&'t str, &'t str> = match self.algorithm.method {
            Method::Inclusive => {
                let parent_scope = parent.map_or(&empty, |frame| &*frame.in_scope);
                inclusive_declarations(&in_scope, parent_scope)
            }
            Method::Exclusive => {
                let wanted = exclusive_declarations(
                    element,
                    &in_scope,
                    &rendered,
                    self.algorithm.inclusive_prefixes,
                );
                if !wanted.is_empty() {
                    let mut next = (*rendered).clone();
                    next.extend(wanted.iter().map(|(&prefix, &uri)| (prefix, uri)));
                    rendered = Rc::new(next);
                }
                wanted
            }
        };

        let mut attributes: Vec<&'t Attribute> = element.attributes.iter().collect();
        if inherit && self.algorithm.method == Method::Inclusive {
            for ancestor in self.tree.lineage(id).skip(1) {
                for attribute in &ancestor.attributes {
                    if attribute.namespace == XML_NAMESPACE
                        && !attributes.iter().any(|present| {
                            present.namespace == XML_NAMESPACE && present.local == attribute.local
                        })
                    {
                        attributes.push(attribute);
                    }
                }
            }
        }
        attributes.sort_by(|left, right| {
            (left.namespace.as_str(), left.local.as_str())
                .cmp(&(right.namespace.as_str(), right.local.as_str()))
        });

        self.out.push('<');
        self.out.push_str(&element.name);
        for (prefix, uri) in declarations {
            self.out.push_str(" xmlns");
            if !prefix.is_empty() {
                self.out.push(':');
                self.out.push_str(prefix);
            }
            self.out.push_str("=\"");
            escape_attribute(&mut self.out, uri);
            self.out.push('"');
        }
        for attribute in attributes {
            self.out.push(' ');
            self.out.push_str(&attribute.name);
            self.out.push_str("=\"");
            escape_attribute(&mut self.out, &attribute.value);
            self.out.push('"');
        }
        self.out.push('>');
        Frame { in_scope, rendered }
    }
}

/// The namespace declarations Canonical `XML` 1.0 renders on an element.
/// Every namespace in scope that the output parent does not have with the
/// same value, plus `xmlns=""` when the element has no default namespace
/// and the output parent has one.
fn inclusive_declarations<'t>(
    in_scope: &BTreeMap<&'t str, &'t str>,
    parent: &BTreeMap<&'t str, &'t str>,
) -> BTreeMap<&'t str, &'t str> {
    let mut declarations = BTreeMap::new();
    for (&prefix, &uri) in in_scope {
        if !uri.is_empty() && parent.get(prefix) != Some(&uri) {
            declarations.insert(prefix, uri);
        }
    }
    let own_default = in_scope.get("").copied().unwrap_or("");
    let parent_default = parent.get("").copied().unwrap_or("");
    if own_default.is_empty() && !parent_default.is_empty() {
        declarations.insert("", "");
    }
    declarations
}

/// The namespace declarations exclusive canonicalization renders on an
/// element. The prefixes the element and its attributes use, and those of
/// the `InclusiveNamespaces PrefixList` that are in scope, each rendered
/// when no output ancestor already rendered it with the same value.
fn exclusive_declarations<'t>(
    element: &'t Element,
    in_scope: &BTreeMap<&'t str, &'t str>,
    rendered: &BTreeMap<&'t str, &'t str>,
    inclusive_prefixes: &[&str],
) -> BTreeMap<&'t str, &'t str> {
    let mut declarations = BTreeMap::new();
    let mut consider = |prefix: &str| {
        // The rendered prefixes are a subset of the ones in scope, so a
        // prefix out of scope has nothing to override.
        let Some((&prefix, &uri)) = in_scope.get_key_value(prefix) else {
            return;
        };
        let current = rendered.get(prefix).copied();
        let needed = if uri.is_empty() {
            current.is_some_and(|value| !value.is_empty())
        } else {
            current != Some(uri)
        };
        if needed {
            declarations.insert(prefix, uri);
        }
    };
    consider(split_name(&element.name).0.unwrap_or(""));
    for attribute in &element.attributes {
        if let (Some(prefix), _) = split_name(&attribute.name)
            && prefix != "xml"
        {
            consider(prefix);
        }
    }
    for &prefix in inclusive_prefixes {
        consider(prefix);
    }
    declarations
}

/// Appends `text` escaped for a canonical text node.
fn escape_text(out: &mut String, text: &str) {
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            other => out.push(other),
        }
    }
}

/// Appends `value` escaped for a canonical attribute value.
fn escape_attribute(out: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            other => out.push(other),
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing case"
)]
mod tests {
    use super::*;

    /// Canonical XML 1.0 without comments.
    const INCLUSIVE: Canonicalization<'static> = Canonicalization {
        method: Method::Inclusive,
        comments: false,
        inclusive_prefixes: &[],
    };

    /// Exclusive canonicalization without comments.
    const EXCLUSIVE: Canonicalization<'static> = Canonicalization {
        method: Method::Exclusive,
        comments: false,
        inclusive_prefixes: &[],
    };

    /// The canonical form of the whole of `input`.
    fn whole(input: &[u8], algorithm: Canonicalization<'_>) -> String {
        let tree = Tree::parse(input).expect("the test document parses");
        String::from_utf8(canonicalize(
            &tree,
            Subset::Document { excluded: &[] },
            algorithm,
        ))
        .expect("canonical output is UTF-8")
    }

    /// The first element named `local` in document order.
    fn find(tree: &Tree, local: &str) -> NodeId {
        (0..tree.nodes.len())
            .find(|&id| {
                tree.element(id)
                    .is_some_and(|element| element.local == local)
            })
            .expect("the test document has the element")
    }

    /// The canonical form of the subtree of the first element named `local`.
    fn apex(input: &[u8], local: &str, algorithm: Canonicalization<'_>) -> String {
        let tree = Tree::parse(input).expect("the test document parses");
        let apex = find(&tree, local);
        String::from_utf8(canonicalize(
            &tree,
            Subset::Element {
                apex,
                excluded: &[],
            },
            algorithm,
        ))
        .expect("canonical output is UTF-8")
    }

    /// Both algorithms with and without comments, for the cases where they
    /// agree.
    fn all() -> [Canonicalization<'static>; 4] {
        [
            INCLUSIVE,
            Canonicalization {
                comments: true,
                ..INCLUSIVE
            },
            EXCLUSIVE,
            Canonicalization {
                comments: true,
                ..EXCLUSIVE
            },
        ]
    }

    #[test]
    fn namespaces_sort_by_prefix_and_attributes_by_namespace_then_name() {
        let input = br#"<a xmlns:z="urn:z" xmlns:b="urn:b" b:y="1" z:x="2" c="3" a="4" xmlns="urn:d"><b:k z:q="9"/></a>"#;
        for algorithm in all() {
            assert_eq!(
                whole(input, algorithm),
                r#"<a xmlns="urn:d" xmlns:b="urn:b" xmlns:z="urn:z" a="4" c="3" b:y="1" z:x="2"><b:k z:q="9"></b:k></a>"#
            );
        }
    }

    #[test]
    fn text_and_attribute_values_are_escaped_after_normalization() {
        let input = b"<a t=\"&lt;&amp;&gt;&quot;&apos;&#9;&#10;&#13; tab\tnl\ncr\" >x &lt; &amp; &gt; &#13; \"q\" 'a'<![CDATA[<&>]]></a>";
        for algorithm in all() {
            assert_eq!(
                whole(input, algorithm),
                "<a t=\"&lt;&amp;>&quot;'&#x9;&#xA;&#xD; tab nl cr\">x &lt; &amp; &gt; &#xD; \"q\" 'a'&lt;&amp;&gt;</a>"
            );
        }
    }

    #[test]
    fn line_ends_become_line_feeds() {
        for algorithm in all() {
            assert_eq!(
                whole(b"<a>one\r\ntwo\rthree\n</a>", algorithm),
                "<a>one\ntwo\nthree\n</a>"
            );
        }
    }

    #[test]
    fn top_level_nodes_sit_on_their_own_lines_and_comments_follow_the_algorithm() {
        let input = b"<?xml version=\"1.0\"?>\n<?pi  data here ?>\n<!-- c1 -->\n<a><!-- inner --><?p?></a>\n<!-- c2 -->\n<?end?>\n";
        for method in [Method::Inclusive, Method::Exclusive] {
            let without = Canonicalization {
                method,
                ..INCLUSIVE
            };
            let with = Canonicalization {
                comments: true,
                ..without
            };
            assert_eq!(
                whole(input, without),
                "<?pi data here ?>\n<a><?p?></a>\n<?end?>"
            );
            assert_eq!(
                whole(input, with),
                "<?pi data here ?>\n<!-- c1 -->\n<a><!-- inner --><?p?></a>\n<!-- c2 -->\n<?end?>"
            );
        }
    }

    #[test]
    fn an_undeclared_default_namespace_is_written_as_an_empty_declaration() {
        let input = br#"<a xmlns="urn:a"><b xmlns=""><c xmlns="urn:a"/></b></a>"#;
        for algorithm in all() {
            assert_eq!(
                whole(input, algorithm),
                r#"<a xmlns="urn:a"><b xmlns=""><c xmlns="urn:a"></c></b></a>"#
            );
        }
    }

    #[test]
    fn redeclarations_are_dropped_and_exclusive_keeps_only_used_prefixes() {
        let input =
            br#"<a xmlns:p="urn:p" xmlns:q="urn:q"><p:b xmlns:p="urn:p" xmlns:q="urn:other"/></a>"#;
        assert_eq!(
            whole(input, INCLUSIVE),
            r#"<a xmlns:p="urn:p" xmlns:q="urn:q"><p:b xmlns:q="urn:other"></p:b></a>"#
        );
        assert_eq!(
            whole(input, EXCLUSIVE),
            r#"<a><p:b xmlns:p="urn:p"></p:b></a>"#
        );
    }

    #[test]
    fn empty_elements_get_end_tags_and_white_space_in_tags_goes() {
        for algorithm in all() {
            assert_eq!(
                whole(b"<a>\n  <b/>\n  <c   attr = \"v\" ></c>\n</a>", algorithm),
                "<a>\n  <b></b>\n  <c attr=\"v\"></c>\n</a>"
            );
        }
    }

    #[test]
    fn the_declared_encoding_is_read_and_utf8_is_written() {
        let input = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><a v=\"\xe8\">citt\xe0</a>";
        assert_eq!(whole(input, INCLUSIVE), "<a v=\"\u{e8}\">citt\u{e0}</a>");
    }

    /// A subtree whose apex inherits namespaces and `xml:` attributes from
    /// ancestors outside the node set.
    const NESTED: &[u8] = br#"<r xmlns="urn:r" xmlns:a="urn:a" xmlns:u="urn:u" xml:lang="it" xml:space="preserve"><a:mid xml:lang="en"><a:apex u:at="1"><inner/></a:apex></a:mid></r>"#;

    #[test]
    fn an_inclusive_subtree_carries_every_inherited_namespace_and_xml_attribute() {
        assert_eq!(
            apex(NESTED, "apex", INCLUSIVE),
            r#"<a:apex xmlns="urn:r" xmlns:a="urn:a" xmlns:u="urn:u" xml:lang="en" xml:space="preserve" u:at="1"><inner></inner></a:apex>"#
        );
    }

    #[test]
    fn an_exclusive_subtree_declares_namespaces_where_they_are_used() {
        assert_eq!(
            apex(NESTED, "apex", EXCLUSIVE),
            r#"<a:apex xmlns:a="urn:a" xmlns:u="urn:u" u:at="1"><inner xmlns="urn:r"></inner></a:apex>"#
        );
    }

    #[test]
    fn the_prefix_list_renders_its_namespaces_at_the_apex() {
        let algorithm = Canonicalization {
            inclusive_prefixes: &[""],
            ..EXCLUSIVE
        };
        assert_eq!(
            apex(NESTED, "apex", algorithm),
            r#"<a:apex xmlns="urn:r" xmlns:a="urn:a" xmlns:u="urn:u" u:at="1"><inner></inner></a:apex>"#
        );
    }

    #[test]
    fn excluded_subtrees_leave_no_trace() {
        let input = br#"<r xmlns:s="urn:s"><x>1</x><s:Sig><s:in/></s:Sig><y>2</y></r>"#;
        let tree = Tree::parse(input).expect("the test document parses");
        let signature = find(&tree, "Sig");
        let excluded = [signature];
        let write = |algorithm| {
            String::from_utf8(canonicalize(
                &tree,
                Subset::Document {
                    excluded: &excluded,
                },
                algorithm,
            ))
            .expect("canonical output is UTF-8")
        };
        assert_eq!(
            write(INCLUSIVE),
            r#"<r xmlns:s="urn:s"><x>1</x><y>2</y></r>"#
        );
        assert_eq!(write(EXCLUSIVE), "<r><x>1</x><y>2</y></r>");
    }

    #[test]
    fn an_apex_inside_an_excluded_subtree_writes_nothing() {
        let input = br#"<r><s:Sig xmlns:s="urn:s"><s:in>1</s:in></s:Sig></r>"#;
        let tree = Tree::parse(input).expect("the test document parses");
        let excluded = [find(&tree, "Sig")];
        let apex = find(&tree, "in");
        for algorithm in all() {
            assert_eq!(
                canonicalize(
                    &tree,
                    Subset::Element {
                        apex,
                        excluded: &excluded
                    },
                    algorithm
                ),
                b""
            );
        }
    }

    #[test]
    fn a_doctype_is_refused() {
        let error = Tree::parse(b"<!DOCTYPE a [<!ENTITY e \"x\">]><a>&e;</a>")
            .expect_err("a DOCTYPE is refused");
        assert!(matches!(error, TreeError::Doctype), "{error:?}");
    }

    #[test]
    fn a_document_that_is_not_well_formed_is_malformed() {
        for input in [
            &b"<a>&e;</a>"[..],
            b"<p:a/>",
            b"<a p:x=\"1\"/>",
            b"<r xmlns:p=\"\"><p:a/></r>",
            b"<a></b>",
            b"<a>&#xZZ;</a>",
            b"<a x=\"&e;\"/>",
            b"<a/><b/>",
            b"<!-- no element -->",
            b"<a><b/>",
        ] {
            let error = Tree::parse(input).expect_err("the input is not well-formed");
            assert!(matches!(error, TreeError::Malformed(_)), "{error:?}");
        }
    }

    /// A declaration of the `xml` prefix binds what is already bound, so no
    /// algorithm renders it.
    #[test]
    fn a_declaration_of_the_xml_prefix_is_not_rendered() {
        for algorithm in all() {
            assert_eq!(
                whole(
                    br#"<a xmlns:xml="http://www.w3.org/XML/1998/namespace" xml:lang="it"/>"#,
                    algorithm
                ),
                r#"<a xml:lang="it"></a>"#
            );
        }
    }

    #[test]
    fn the_text_of_an_element_skips_its_comments_and_child_elements() {
        let tree = Tree::parse(b"<a>x<!--c-->y<b>w</b>z</a>").expect("the test document parses");
        assert_eq!(tree.text(find(&tree, "a")), "xyz");
    }

    #[test]
    fn every_predefined_entity_resolves() {
        for algorithm in all() {
            assert_eq!(
                whole(
                    b"<a v=\"&lt;&gt;&amp;&apos;&quot;\">&lt;&gt;&amp;&apos;&quot;</a>",
                    algorithm
                ),
                "<a v=\"&lt;>&amp;'&quot;\">&lt;&gt;&amp;'\"</a>"
            );
        }
    }

    /// An unprefixed attribute has no namespace even under a default one, so
    /// it sorts before every namespaced attribute.
    #[test]
    fn unprefixed_attributes_stay_outside_the_default_namespace() {
        for algorithm in all() {
            assert_eq!(
                whole(
                    br#"<a:r xmlns:a="urn:a" xmlns="urn:z"><a:e a:c="2" b="1"/></a:r>"#,
                    algorithm
                ),
                match algorithm.method {
                    Method::Inclusive =>
                        r#"<a:r xmlns="urn:z" xmlns:a="urn:a"><a:e b="1" a:c="2"></a:e></a:r>"#,
                    Method::Exclusive => r#"<a:r xmlns:a="urn:a"><a:e b="1" a:c="2"></a:e></a:r>"#,
                }
            );
        }
    }
}
