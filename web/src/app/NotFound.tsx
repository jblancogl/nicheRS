import { Link } from "react-router";

import { Panel } from "@/components/domain/Panel";
import { Button } from "@/components/ui/button";

import { useShellMessages } from "./messages";

export function NotFound() {
  const m = useShellMessages();
  return (
    <Panel title={m.notFound}>
      <p className="text-sm text-muted-foreground">{m.notFoundText}</p>
      <Button className="mt-3" render={<Link to="/" />} nativeButton={false}>
        {m.goHome}
      </Button>
    </Panel>
  );
}
