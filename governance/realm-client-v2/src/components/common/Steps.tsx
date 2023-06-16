import React from "react";

interface StepProps {
  step: number;
  content: string;
}
export default function Steps(props: StepProps) {
  const { step = "1", content = "Rules & Options" } = props;

  return (
    <div className="sticky bg-neutral-900 p-3 text-neutral-600 flex justify-center gap-2 my-1">
      Step {step} of 2 <span className="text-neutral-300">{content} </span>
    </div>
  );
}
