use burn::{
    module::Module,
    nn::{loss::CrossEntropyLossConfig, Dropout, DropoutConfig, Linear, LinearConfig, Relu},
    prelude::Backend,
    tensor::{backend::AutodiffBackend, DataError, Tensor, TensorData},
    train::{ClassificationOutput, TrainOutput, TrainStep, ValidStep},
};

use crate::dataset::{DailyLinearBatch, DailyLinearInferBatch};

const INPUT_SIZE: usize = 23;
const HIDDEN_SIZES: [usize; 6] = [64, 128, 256, 512, 1024, 2048];
const OUTPUT_SIZE: usize = 2;

#[derive(Module, Debug)]
pub struct Model<B: Backend> {
    input_layer: Linear<B>,
    ln1: Linear<B>,
    ln2: Linear<B>,
    ln3: Linear<B>,
    output_layer: Linear<B>,
    dropout: Dropout,
    activation: Relu,
}

impl<B: Backend> Default for Model<B> {
    fn default() -> Self {
        let device = B::Device::default();
        Self::new(&device)
    }
}

impl<B: Backend> Model<B> {
    pub fn new(device: &B::Device) -> Self {
        let h1 = HIDDEN_SIZES[0];
        let h2 = HIDDEN_SIZES[1];
        let h3 = HIDDEN_SIZES[2];
        let input_layer = LinearConfig::new(INPUT_SIZE, h1)
            .with_bias(true)
            .init(device);

        let ln1 = LinearConfig::new(h1, h2).with_bias(true).init(device);

        let ln2 = LinearConfig::new(h2, h2).with_bias(true).init(device);

        let ln3 = LinearConfig::new(h2, h3).with_bias(true).init(device);

        let output_layer = LinearConfig::new(h3, OUTPUT_SIZE)
            .with_bias(true)
            .init(device);

        let dropout = DropoutConfig::new(0.5).init();
        let activation = Relu::new();

        Self {
            input_layer,
            ln1,
            ln2,
            ln3,
            output_layer,
            dropout,
            activation,
        }
    }

    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = input.detach();
        let x = self.input_layer.forward(x);
        let x = self.activation.forward(x);
        let x = self.dropout.forward(x);

        let x = self.ln1.forward(x);
        let x = self.activation.forward(x);
        let x = self.dropout.forward(x);

        let x = self.ln2.forward(x);
        let x = self.activation.forward(x);
        let x = self.dropout.forward(x);

        let x = self.ln3.forward(x);
        let x = self.activation.forward(x);
        let x = self.dropout.forward(x);

        self.output_layer.forward(x)
    }

    pub fn forward_step(&self, item: DailyLinearBatch<B>) -> ClassificationOutput<B> {
        let targets = item.targets;
        let output = self.forward(item.inputs);

        let loss = CrossEntropyLossConfig::new()
            .with_logits(true)
            .init(&output.device())
            .forward(output.clone(), targets.clone()); // bce loss requires targets to be of shape

        ClassificationOutput {
            loss,
            output,
            targets,
        }
    }

    pub fn infer(&self, item: DailyLinearInferBatch<B>) -> Tensor<B, 2> {
        let output = self.forward(item.inputs);
        output
    }
}

impl<B: AutodiffBackend> TrainStep<DailyLinearBatch<B>, ClassificationOutput<B>> for Model<B> {
    fn step(&self, item: DailyLinearBatch<B>) -> TrainOutput<ClassificationOutput<B>> {
        let item = self.forward_step(item);

        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> ValidStep<DailyLinearBatch<B>, ClassificationOutput<B>> for Model<B> {
    fn step(&self, item: DailyLinearBatch<B>) -> ClassificationOutput<B> {
        self.forward_step(item)
    }
}
