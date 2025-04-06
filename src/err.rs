use std::{
    error::Error,
    ops::{ControlFlow, FromResidual, Try},
};

#[derive(Debug)]
pub enum Tracy<T> {
    Ok(T),
    Err(Trace),
}
impl<T, E> From<Result<T, E>> for Tracy<T> {
    fn from(_: Result<T, E>) -> Self {
        todo!()
    }
}
impl<T> Try for Tracy<T> {
    type Output = T;

    type Residual = Tracy<Trace>;

    fn from_output(output: Self::Output) -> Self {
        Tracy::Ok(output)
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Tracy::Ok(t) => ControlFlow::Continue(t),
            Tracy::Err(trace) => ControlFlow::Break(Tracy::Err(trace)),
        }
    }
}
impl<T> FromResidual for Tracy<T> {
    fn from_residual(residual: <Self as std::ops::Try>::Residual) -> Self {
        match residual {
            Tracy::Ok(_) => todo!(),
            Tracy::Err(trace) => Tracy::Err(trace),
        }
    }
}

#[derive(Debug)]
pub struct Trace(pub Vec<TraceEntry>);
impl<E: Error + 'static> From<E> for Trace {
    fn from(value: E) -> Self {
        Self(vec![TraceEntry::from(value)])
    }
}

#[derive(Debug)]
pub struct TraceEntry {
    error: Box<dyn Error>,
    file: &'static str,
    col: usize,
    row: usize,
}
impl<E: Error + 'static> From<E> for TraceEntry {
    fn from(value: E) -> Self {
        Self {
            error: Box::new(value),
            file: "///",
            col: 0,
            row: 0,
        }
    }
}
