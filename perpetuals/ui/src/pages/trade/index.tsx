import React, { useEffect } from "react";
import Router from "next/router";

const IndexPage = () => {
  useEffect(() => {
    const { pathname } = Router;
    if (pathname == "/trade") {
      Router.push("/trade/SOL-USD");
    }
  });

  return <></>;
};

export default IndexPage;
