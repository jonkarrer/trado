use burn::{
    module::Module,
    nn::{
        loss::MseLoss, Dropout, DropoutConfig, LayerNorm, LayerNormConfig, Linear, LinearConfig,
        Relu,
    },
    prelude::Backend,
    tensor::{backend::AutodiffBackend, DataError, Tensor},
    train::{RegressionOutput, TrainOutput, TrainStep, ValidStep},
};

use crate::dataset::{DailyLinearBatch, DailyLinearInferBatch};

const INPUT_SIZE: usize = 47;
const HIDDEN_SIZES: [usize; 3] = [64, 128, 256];
const OUTPUT_SIZE: usize = 1;

#[derive(Module, Debug)]
pub struct Model<B: Backend> {
    input_layer: Linear<B>,
    ln1: Linear<B>,
    ln2: Linear<B>,
    ln3: Linear<B>,
    output_layer: Linear<B>,
    dropout: Dropout,
    activation: Relu,
    layer_norm: LayerNorm<B>,
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
        let input_layer = LinearConfig::new(INPUT_SIZE, h1)
            .with_bias(true)
            .init(device);

        let ln1 = LinearConfig::new(h1, h1).with_bias(true).init(device);
        let ln2 = LinearConfig::new(h1, h2).with_bias(true).init(device);
        let ln3 = LinearConfig::new(h2, h2).with_bias(true).init(device);

        let output_layer = LinearConfig::new(h2, OUTPUT_SIZE)
            .with_bias(true)
            .init(device);

        let dropout = DropoutConfig::new(0.33).init();
        let activation = Relu::new();
        let layer_norm = LayerNormConfig::new(h1).init(device);

        Self {
            input_layer,
            ln1,
            ln2,
            ln3,
            output_layer,
            dropout,
            activation,
            layer_norm,
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

    pub fn forward_step(&self, item: DailyLinearBatch<B>) -> RegressionOutput<B> {
        let targets: Tensor<B, 2> = item.targets.unsqueeze();
        let output: Tensor<B, 2> = self.forward(item.inputs);

        let loss = MseLoss::new().forward(
            output.clone(),
            targets.clone(),
            burn::nn::loss::Reduction::Mean,
        ); // bce loss requires targets to be of shape

        RegressionOutput {
            loss,
            output,
            targets,
        }
    }

    pub fn infer(&self, item: DailyLinearInferBatch<B>) -> Result<Vec<f32>, DataError> {
        let output = self.forward(item.inputs);
        dbg!(output.to_data());
        output.to_data().to_vec()
    }
}

impl<B: AutodiffBackend> TrainStep<DailyLinearBatch<B>, RegressionOutput<B>> for Model<B> {
    fn step(&self, item: DailyLinearBatch<B>) -> TrainOutput<RegressionOutput<B>> {
        let item = self.forward_step(item);

        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> ValidStep<DailyLinearBatch<B>, RegressionOutput<B>> for Model<B> {
    fn step(&self, item: DailyLinearBatch<B>) -> RegressionOutput<B> {
        self.forward_step(item)
    }
}
