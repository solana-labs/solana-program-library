import NavBar from "@/components/common/NavBar";
import Steps from "@/components/common/Steps";
import Proposal_Summary from "@/components/poll/Proposal_summary";
import React from "react";

export default function ProposalSummary() {
  return (
    <main className=" w-full h-screen">
      <NavBar />
      <Steps step={2} content={"Title & Description"} />
      <Proposal_Summary />
    </main>
  );
}
