use descartes::{P2};
use kay::{Swarm, Recipient, CreateWith, ActorSystem, Individual, Fate};
use monet::{Norm};

mod plan;
mod road_stroke;
mod road_stroke_node_interactable;
mod road_stroke_canvas;
pub mod materialized_reality;
pub mod current_plan_rendering;

pub use self::plan::{Plan, RoadStrokeRef, IntersectionRef, InbetweenStrokeRef, PlanDelta, PlanResult, PlanResultDelta};
pub use self::road_stroke::{RoadStroke, RoadStrokeNode, RoadStrokeNodeRef};
pub use self::road_stroke_node_interactable::RoadStrokeNodeInteractable;
pub use self::road_stroke_canvas::RoadStrokeCanvas;
use self::materialized_reality::MaterializedReality;

#[derive(Compact, Clone, Default)]
pub struct CurrentPlan {
    delta: PlanDelta,
    pub current_plan_result: PlanResult,
    pub current_plan_result_delta: PlanResultDelta,
    ui_state: PlanUIState
}
impl Individual for CurrentPlan{}

#[derive(Copy, Clone)]
enum PlanControl{
    AddRoadStrokeNode(P2),
    MoveRoadStrokeNodeTo(RoadStrokeNodeRef, P2),
    Materialize
}

use self::materialized_reality::Simulate;
use self::materialized_reality::Apply;

impl Recipient<PlanControl> for CurrentPlan {
    fn receive(&mut self, msg: &PlanControl) -> Fate {match *msg{
        PlanControl::AddRoadStrokeNode(at) => {
            let new_node = RoadStrokeNode{position: at, direction: None};
            
            if let Some(RoadStrokeNodeRef(stroke_idx, node_idx)) = self.ui_state.current_node {
                let stroke = &mut self.delta.new_strokes[stroke_idx];
                let current_node = stroke.nodes[node_idx];

                if (current_node.position - new_node.position).norm() < 5.0 {
                    // finish stroke
                    self.ui_state.current_node = None;
                } else if node_idx == stroke.nodes.len() - 1 {
                    // append
                    stroke.nodes.push(new_node);
                    self.ui_state.current_node = Some(RoadStrokeNodeRef(stroke_idx, stroke.nodes.len() - 1));
                } else if node_idx == 0 {
                    // prepend
                    stroke.nodes.insert(0, new_node);
                } else {unreachable!()}
            } else {
                self.delta.new_strokes.push(RoadStroke{
                    nodes: vec![new_node].into()
                });
                self.ui_state.current_node = Some(RoadStrokeNodeRef(self.delta.new_strokes.len() - 1, 0))
            }

            MaterializedReality::id() << Simulate{requester: CurrentPlan::id(), delta: self.delta.clone()};
            Fate::Live
        },
        PlanControl::MoveRoadStrokeNodeTo(RoadStrokeNodeRef(stroke, node), position) =>  {
            self.delta.new_strokes[stroke].nodes[node].position = position;
            MaterializedReality::id() << Simulate{requester: CurrentPlan::id(), delta: self.delta.clone()};
            Fate::Live
        },
        PlanControl::Materialize => {
            MaterializedReality::id() << Apply{delta: self.delta.clone()};
            *self = CurrentPlan::default();
            Self::id() << RecreateInteractables;
            Fate::Live
        }
    }}
}

use self::materialized_reality::SimulationResult;

impl Recipient<SimulationResult> for CurrentPlan{
    fn receive(&mut self, msg: &SimulationResult) -> Fate {match *msg{
        SimulationResult{ref result, ref result_delta} => {
            self.current_plan_result = result.clone();
            self.current_plan_result_delta = result_delta.clone();
            self.ui_state.dirty = true;
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
struct RecreateInteractables;
#[derive(Copy, Clone)]
struct ClearAll;

impl Recipient<RecreateInteractables> for CurrentPlan {
    fn receive(&mut self, _msg: &RecreateInteractables) -> Fate {
        Swarm::<RoadStrokeNodeInteractable>::all() << ClearAll;
        Swarm::<RoadStrokeCanvas>::all() << ClearAll;
        self.create_interactables();
        Fate::Live
    }
}

impl CurrentPlan{
    fn create_interactables(&self) {
        Swarm::<RoadStrokeCanvas>::all() << CreateWith(RoadStrokeCanvas::new(), AddToUI);
        for (i, stroke) in self.delta.new_strokes.iter().enumerate() {
            stroke.create_interactables(RoadStrokeRef(i));
        }
    }
}

#[derive(Compact, Clone)]
struct PlanUIState{
    current_node: Option<RoadStrokeNodeRef>,
    dirty: bool
}

impl Default for PlanUIState{
    fn default() -> PlanUIState{
        PlanUIState{current_node: None, dirty: true}
    }
}

#[derive(Copy, Clone)]
struct AddToUI;

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(CurrentPlan::default());
    system.add_inbox::<PlanControl, CurrentPlan>();
    system.add_inbox::<SimulationResult, CurrentPlan>();
    system.add_inbox::<RecreateInteractables, CurrentPlan>();
    self::materialized_reality::setup(system);
    self::road_stroke_node_interactable::setup(system);
    self::road_stroke_canvas::setup(system);
}