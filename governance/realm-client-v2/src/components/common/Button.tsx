import React, { ReactElement, ReactNode } from "react";
import { Button as ChakraButton } from "@chakra-ui/button";
import { Icon } from "@chakra-ui/icon";

interface ButtonProps {
  type: "submit" | "reset" | "button";
  text: string;
  onClick?: () => void;
  variant?: "solid" | "outline" | "ghost" | "link";
  leftIcon?: ReactElement;
  rightIcon?: ReactElement;
  rounded?: boolean;
  colorScheme?: "brand" | string;
  className?: string;
}

export default function Button(props: ButtonProps) {
  const {
    type,
    text,
    onClick,
    variant,
    leftIcon,
    rightIcon,
    rounded,
    colorScheme,
    className,
  } = props;
  const roundedStyle = rounded === true ? "full" : "";
  const defaults: Pick<ButtonProps, "colorScheme" | "className"> = {
    colorScheme: "brand",
    className: "w-full rounded",
  };

  return (
    <ChakraButton
      {...defaults}
      onClick={onClick}
      type={type}
      rounded={roundedStyle}
      className={className}
      variant={variant}
      leftIcon={leftIcon}
      rightIcon={rightIcon}
    >
      {text}
    </ChakraButton>
  );
}
