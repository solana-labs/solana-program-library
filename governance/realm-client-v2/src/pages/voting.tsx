import NavBar from "@/components/common/NavBar";
import ProfileBanner from "@/components/common/ProfileBanner";
import UserTabs from "@/components/common/UserTabs";
import React from "react";

export default function Voting() {
  return (
    <main className="mx-auto">
      <NavBar />
      <ProfileBanner />
      <UserTabs />
    </main>
  );
}
