use crate::{is_blacklisted_instruction, is_variant_instruction, relocatable_regions};
use binaryninja::architecture::CoreArchitecture;
use binaryninja::basic_block::BasicBlock;
use binaryninja::disassembly::DisassemblyTextLine;
use binaryninja::flowgraph::FlowGraph;
use binaryninja::function::{HighlightColor, HighlightStandardColor, NativeBlock};
use binaryninja::low_level_il::instruction::LowLevelInstructionIndex;
use binaryninja::low_level_il::RegularLowLevelILFunction;
use binaryninja::render_layer::{register_render_layer, RenderLayer};

// TODO: Add a render layer to show basic block GUID's?

pub struct HighlightRenderLayer {
    blacklist: HighlightColor,
    variant: HighlightColor,
}

impl HighlightRenderLayer {
    pub fn register() {
        register_render_layer(
            "WARP Highlight Layer",
            // TODO: Make the highlight colors configurable.
            HighlightRenderLayer {
                blacklist: HighlightColor::StandardHighlightColor {
                    color: HighlightStandardColor::OrangeHighlightColor,
                    alpha: 155,
                },
                variant: HighlightColor::StandardHighlightColor {
                    color: HighlightStandardColor::RedHighlightColor,
                    alpha: 155,
                },
            },
            Default::default(),
        );
    }

    /// Highlights the lines that are variant or blacklisted.
    pub fn highlight_lines(
        &self,
        llil: &RegularLowLevelILFunction<CoreArchitecture>,
        lines: &mut [DisassemblyTextLine],
    ) {
        // TODO: Calling relocatable regions here is scuffed, this should probably be passed in...
        let relocatable_regions = relocatable_regions(&llil.function().view());
        for line in lines {
            let llil_instr_idx = LowLevelInstructionIndex(line.instruction_index);
            if let Some(llil_instr) = llil.instruction_from_index(llil_instr_idx) {
                if is_blacklisted_instruction(&llil_instr) {
                    line.highlight = self.blacklist;
                } else if is_variant_instruction(&relocatable_regions, &llil_instr) {
                    line.highlight = self.variant;
                }
            }
        }
    }
}

impl RenderLayer for HighlightRenderLayer {
    fn apply_to_flow_graph(&self, graph: &mut FlowGraph) {
        // TODO: This should only apply to LLIL flow graphs...
        if let Ok(llil) = graph.low_level_il() {
            for node in &graph.nodes() {
                let mut new_lines = node.lines().to_vec();
                self.highlight_lines(&llil, &mut new_lines);
                node.set_lines(new_lines);
            }
        }
    }

    fn apply_to_llil_block(
        &self,
        block: &BasicBlock<NativeBlock>,
        mut lines: Vec<DisassemblyTextLine>,
    ) -> Vec<DisassemblyTextLine> {
        // Highlight any LLIL instruction that will be masked by WARP.
        let function = block.function();
        if let Ok(llil) = function.low_level_il() {
            self.highlight_lines(&llil, &mut lines);
        }
        lines
    }
}
