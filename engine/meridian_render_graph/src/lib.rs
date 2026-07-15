//! Renderer-independent render-graph declaration and validation.

use std::collections::{BTreeSet, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceId(usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PassId(usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    Buffer,
    Texture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceDescriptor {
    pub name: String,
    pub kind: ResourceKind,
    /// Imported resources exist before this graph begins execution.
    pub imported: bool,
    /// Transient resources may eventually be aliased when lifetimes do not overlap.
    pub transient: bool,
}

impl ResourceDescriptor {
    #[must_use]
    pub fn imported(name: impl Into<String>, kind: ResourceKind) -> Self {
        Self {
            name: name.into(),
            kind,
            imported: true,
            transient: false,
        }
    }

    #[must_use]
    pub fn transient(name: impl Into<String>, kind: ResourceKind) -> Self {
        Self {
            name: name.into(),
            kind,
            imported: false,
            transient: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLifetime {
    pub first_pass_index: usize,
    pub last_pass_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PassDescriptor {
    name: String,
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
    depends_on: Vec<PassId>,
}

#[derive(Clone, Debug, Default)]
pub struct RenderGraphBuilder {
    resources: Vec<ResourceDescriptor>,
    passes: Vec<PassDescriptor>,
}

impl RenderGraphBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_resource(&mut self, descriptor: ResourceDescriptor) -> ResourceId {
        let id = ResourceId(self.resources.len());
        self.resources.push(descriptor);
        id
    }

    pub fn add_pass(
        &mut self,
        name: impl Into<String>,
        reads: impl IntoIterator<Item = ResourceId>,
        writes: impl IntoIterator<Item = ResourceId>,
        depends_on: impl IntoIterator<Item = PassId>,
    ) -> PassId {
        let id = PassId(self.passes.len());
        self.passes.push(PassDescriptor {
            name: name.into(),
            reads: reads.into_iter().collect(),
            writes: writes.into_iter().collect(),
            depends_on: depends_on.into_iter().collect(),
        });
        id
    }

    /// Validates declarations, infers resource hazards, and topologically sorts passes.
    ///
    /// # Errors
    ///
    /// Returns [`RenderGraphError`] when names or IDs are invalid, a resource
    /// is read without a producer, a pass has an unsupported access conflict,
    /// or the resulting dependency graph contains a cycle.
    pub fn compile(self) -> Result<CompiledRenderGraph, RenderGraphError> {
        self.validate_names()?;
        self.validate_ids_and_access()?;

        let mut edges = vec![BTreeSet::new(); self.passes.len()];
        self.add_explicit_edges(&mut edges);
        self.add_resource_edges(&mut edges)?;
        let pass_order = self.topological_order(&edges)?;
        let resource_lifetimes = self.resource_lifetimes(&pass_order);

        Ok(CompiledRenderGraph {
            resources: self.resources,
            passes: self.passes,
            pass_order,
            resource_lifetimes,
        })
    }

    fn validate_names(&self) -> Result<(), RenderGraphError> {
        let mut resource_names = HashSet::new();
        for resource in &self.resources {
            if !resource_names.insert(resource.name.as_str()) {
                return Err(RenderGraphError::DuplicateResourceName(
                    resource.name.clone(),
                ));
            }
        }

        let mut pass_names = HashSet::new();
        for pass in &self.passes {
            if !pass_names.insert(pass.name.as_str()) {
                return Err(RenderGraphError::DuplicatePassName(pass.name.clone()));
            }
        }

        Ok(())
    }

    fn validate_ids_and_access(&self) -> Result<(), RenderGraphError> {
        for (pass_index, pass) in self.passes.iter().enumerate() {
            for resource in pass.reads.iter().chain(&pass.writes) {
                if resource.0 >= self.resources.len() {
                    return Err(RenderGraphError::UnknownResource {
                        pass: pass.name.clone(),
                        resource: *resource,
                    });
                }
            }

            let reads = pass.reads.iter().copied().collect::<HashSet<_>>();
            if let Some(resource) = pass.writes.iter().find(|resource| reads.contains(resource)) {
                return Err(RenderGraphError::ReadWriteConflict {
                    pass: pass.name.clone(),
                    resource: self.resources[resource.0].name.clone(),
                });
            }

            if pass
                .depends_on
                .iter()
                .any(|dependency| dependency.0 >= self.passes.len())
            {
                let dependency = pass
                    .depends_on
                    .iter()
                    .find(|dependency| dependency.0 >= self.passes.len())
                    .copied()
                    .expect("an unknown dependency was found");
                return Err(RenderGraphError::UnknownDependency {
                    pass: PassId(pass_index),
                    dependency,
                });
            }
        }

        Ok(())
    }

    fn add_explicit_edges(&self, edges: &mut [BTreeSet<usize>]) {
        for (pass_index, pass) in self.passes.iter().enumerate() {
            for dependency in &pass.depends_on {
                edges[dependency.0].insert(pass_index);
            }
        }
    }

    fn add_resource_edges(&self, edges: &mut [BTreeSet<usize>]) -> Result<(), RenderGraphError> {
        let mut last_writer = vec![None::<usize>; self.resources.len()];
        let mut readers_since_write = vec![Vec::<usize>::new(); self.resources.len()];

        for (pass_index, pass) in self.passes.iter().enumerate() {
            for resource in &pass.reads {
                match last_writer[resource.0] {
                    Some(writer) => {
                        edges[writer].insert(pass_index);
                    }
                    None if !self.resources[resource.0].imported => {
                        return Err(RenderGraphError::MissingProducer {
                            pass: pass.name.clone(),
                            resource: self.resources[resource.0].name.clone(),
                        });
                    }
                    None => {}
                }
                readers_since_write[resource.0].push(pass_index);
            }

            for resource in &pass.writes {
                if let Some(writer) = last_writer[resource.0] {
                    edges[writer].insert(pass_index);
                }
                for reader in readers_since_write[resource.0].drain(..) {
                    edges[reader].insert(pass_index);
                }
                last_writer[resource.0] = Some(pass_index);
            }
        }

        Ok(())
    }

    fn topological_order(
        &self,
        edges: &[BTreeSet<usize>],
    ) -> Result<Vec<PassId>, RenderGraphError> {
        let mut indegrees = vec![0_usize; self.passes.len()];
        for destinations in edges {
            for destination in destinations {
                indegrees[*destination] += 1;
            }
        }

        let mut ready = indegrees
            .iter()
            .enumerate()
            .filter_map(|(index, indegree)| (*indegree == 0).then_some(index))
            .collect::<BTreeSet<_>>();
        let mut order = Vec::with_capacity(self.passes.len());

        while let Some(pass_index) = ready.pop_first() {
            order.push(PassId(pass_index));
            for destination in &edges[pass_index] {
                indegrees[*destination] -= 1;
                if indegrees[*destination] == 0 {
                    ready.insert(*destination);
                }
            }
        }

        if order.len() != self.passes.len() {
            let passes = indegrees
                .iter()
                .enumerate()
                .filter(|(_, indegree)| **indegree > 0)
                .map(|(index, _)| self.passes[index].name.clone())
                .collect();
            return Err(RenderGraphError::DependencyCycle { passes });
        }

        Ok(order)
    }

    fn resource_lifetimes(&self, order: &[PassId]) -> Vec<Option<ResourceLifetime>> {
        let mut lifetimes = vec![None::<ResourceLifetime>; self.resources.len()];
        for (pass_order_index, pass_id) in order.iter().enumerate() {
            let pass = &self.passes[pass_id.0];
            for resource in pass.reads.iter().chain(&pass.writes) {
                let lifetime = &mut lifetimes[resource.0];
                match lifetime {
                    Some(existing) => existing.last_pass_index = pass_order_index,
                    None => {
                        *lifetime = Some(ResourceLifetime {
                            first_pass_index: pass_order_index,
                            last_pass_index: pass_order_index,
                        });
                    }
                }
            }
        }
        lifetimes
    }
}

#[derive(Clone, Debug)]
pub struct CompiledRenderGraph {
    resources: Vec<ResourceDescriptor>,
    passes: Vec<PassDescriptor>,
    pass_order: Vec<PassId>,
    resource_lifetimes: Vec<Option<ResourceLifetime>>,
}

impl CompiledRenderGraph {
    #[must_use]
    pub fn ordered_passes(&self) -> impl ExactSizeIterator<Item = (PassId, &str)> {
        self.pass_order
            .iter()
            .copied()
            .map(|pass_id| (pass_id, self.passes[pass_id.0].name.as_str()))
    }

    #[must_use]
    pub fn resource_name(&self, resource: ResourceId) -> Option<&str> {
        self.resources
            .get(resource.0)
            .map(|entry| entry.name.as_str())
    }

    #[must_use]
    pub fn resource_lifetime(&self, resource: ResourceId) -> Option<ResourceLifetime> {
        self.resource_lifetimes.get(resource.0).copied().flatten()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderGraphError {
    DuplicateResourceName(String),
    DuplicatePassName(String),
    UnknownResource { pass: String, resource: ResourceId },
    UnknownDependency { pass: PassId, dependency: PassId },
    ReadWriteConflict { pass: String, resource: String },
    MissingProducer { pass: String, resource: String },
    DependencyCycle { passes: Vec<String> },
}

impl Display for RenderGraphError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateResourceName(name) => {
                write!(formatter, "duplicate resource name: {name}")
            }
            Self::DuplicatePassName(name) => write!(formatter, "duplicate pass name: {name}"),
            Self::UnknownResource { pass, resource } => {
                write!(
                    formatter,
                    "pass {pass} references unknown resource {resource:?}"
                )
            }
            Self::UnknownDependency { pass, dependency } => {
                write!(
                    formatter,
                    "pass {pass:?} depends on unknown pass {dependency:?}"
                )
            }
            Self::ReadWriteConflict { pass, resource } => {
                write!(
                    formatter,
                    "pass {pass} reads and writes {resource} in one declaration"
                )
            }
            Self::MissingProducer { pass, resource } => {
                write!(
                    formatter,
                    "pass {pass} reads {resource} before it has a producer"
                )
            }
            Self::DependencyCycle { passes } => {
                write!(
                    formatter,
                    "render graph contains a dependency cycle involving {}",
                    passes.join(", ")
                )
            }
        }
    }
}

impl Error for RenderGraphError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_resource_hazards_and_lifetimes() {
        let mut graph = RenderGraphBuilder::new();
        let swapchain = graph.add_resource(ResourceDescriptor::imported(
            "swapchain",
            ResourceKind::Texture,
        ));
        let depth = graph.add_resource(ResourceDescriptor::transient(
            "depth",
            ResourceKind::Texture,
        ));
        let lighting = graph.add_resource(ResourceDescriptor::transient(
            "lighting",
            ResourceKind::Texture,
        ));

        let depth_pass = graph.add_pass("depth", [], [depth], []);
        let lighting_pass = graph.add_pass("lighting", [depth], [lighting], []);
        let present_pass = graph.add_pass("present", [lighting], [swapchain], []);

        let compiled = graph.compile().expect("graph should compile");
        let order = compiled
            .ordered_passes()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        assert_eq!(order, vec![depth_pass, lighting_pass, present_pass]);
        assert_eq!(
            compiled.resource_lifetime(depth),
            Some(ResourceLifetime {
                first_pass_index: 0,
                last_pass_index: 1,
            })
        );
    }

    #[test]
    fn rejects_read_without_import_or_producer() {
        let mut graph = RenderGraphBuilder::new();
        let depth = graph.add_resource(ResourceDescriptor::transient(
            "depth",
            ResourceKind::Texture,
        ));
        graph.add_pass("lighting", [depth], [], []);

        assert_eq!(
            graph.compile().expect_err("missing producer should fail"),
            RenderGraphError::MissingProducer {
                pass: "lighting".to_owned(),
                resource: "depth".to_owned(),
            }
        );
    }

    #[test]
    fn detects_cycle_between_explicit_and_resource_dependencies() {
        let mut graph = RenderGraphBuilder::new();
        let depth = graph.add_resource(ResourceDescriptor::transient(
            "depth",
            ResourceKind::Texture,
        ));
        let writer = graph.add_pass("writer", [], [depth], [PassId(1)]);
        let reader = graph.add_pass("reader", [depth], [], []);

        let error = graph.compile().expect_err("cycle should fail");
        assert_eq!(
            error,
            RenderGraphError::DependencyCycle {
                passes: vec!["writer".to_owned(), "reader".to_owned()],
            }
        );
        assert_eq!(writer, PassId(0));
        assert_eq!(reader, PassId(1));
    }

    #[test]
    fn rejects_duplicate_names_and_same_pass_read_write() {
        let mut duplicate = RenderGraphBuilder::new();
        duplicate.add_resource(ResourceDescriptor::transient(
            "color",
            ResourceKind::Texture,
        ));
        duplicate.add_resource(ResourceDescriptor::transient(
            "color",
            ResourceKind::Texture,
        ));
        assert_eq!(
            duplicate.compile().expect_err("duplicate should fail"),
            RenderGraphError::DuplicateResourceName("color".to_owned())
        );

        let mut conflict = RenderGraphBuilder::new();
        let color =
            conflict.add_resource(ResourceDescriptor::imported("color", ResourceKind::Texture));
        conflict.add_pass("feedback", [color], [color], []);
        assert_eq!(
            conflict.compile().expect_err("conflict should fail"),
            RenderGraphError::ReadWriteConflict {
                pass: "feedback".to_owned(),
                resource: "color".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_unknown_resource_identifier() {
        let mut graph = RenderGraphBuilder::new();
        graph.add_pass("invalid", [ResourceId(99)], [], []);

        assert_eq!(
            graph.compile().expect_err("unknown resource should fail"),
            RenderGraphError::UnknownResource {
                pass: "invalid".to_owned(),
                resource: ResourceId(99),
            }
        );
    }
}
