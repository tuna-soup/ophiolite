import type {
  ButtonHTMLAttributes,
  HTMLAttributes,
  InputHTMLAttributes,
  TextareaHTMLAttributes
} from "react";
import { cn } from "../lib/utils";

export function Button({
  className,
  variant = "default",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "default" | "secondary" | "ghost" | "outline";
}) {
  return (
    <button
      className={cn(
        "inline-flex items-center justify-center rounded-full px-4 py-2 text-sm font-medium transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-stone-500 disabled:pointer-events-none disabled:opacity-50",
        variant === "default" && "bg-amber-800 text-amber-50 hover:bg-amber-700",
        variant === "secondary" && "bg-stone-200 text-stone-900 hover:bg-stone-300",
        variant === "ghost" && "text-stone-700 hover:bg-stone-100",
        variant === "outline" &&
          "border border-stone-300 bg-white/80 text-stone-900 hover:bg-stone-100",
        className
      )}
      {...props}
    />
  );
}

export function Card({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "rounded-[28px] border border-stone-300/80 bg-white/80 text-stone-900 shadow-[0_18px_48px_rgba(79,57,24,0.08)] backdrop-blur-sm",
        className
      )}
      {...props}
    />
  );
}

export function CardHeader({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("flex flex-col gap-2 p-6", className)} {...props} />;
}

export function CardTitle({ className, ...props }: HTMLAttributes<HTMLHeadingElement>) {
  return <h2 className={cn("text-xl font-semibold tracking-tight", className)} {...props} />;
}

export function CardDescription({
  className,
  ...props
}: HTMLAttributes<HTMLParagraphElement>) {
  return <p className={cn("text-sm text-stone-600", className)} {...props} />;
}

export function CardContent({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("px-6 pb-6", className)} {...props} />;
}

export function Input({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={cn(
        "h-11 w-full rounded-2xl border border-stone-300 bg-white/90 px-4 py-2 text-sm shadow-sm outline-none transition placeholder:text-stone-400 focus:ring-2 focus:ring-amber-700/30",
        className
      )}
      {...props}
    />
  );
}

export function Textarea({
  className,
  ...props
}: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={cn(
        "min-h-[120px] w-full rounded-2xl border border-stone-300 bg-white/90 px-4 py-3 text-sm shadow-sm outline-none transition placeholder:text-stone-400 focus:ring-2 focus:ring-amber-700/30",
        className
      )}
      {...props}
    />
  );
}

export function Label({ className, ...props }: HTMLAttributes<HTMLLabelElement>) {
  return <label className={cn("grid gap-2 text-sm font-medium", className)} {...props} />;
}

export function Badge({
  className,
  tone = "default",
  ...props
}: HTMLAttributes<HTMLSpanElement> & { tone?: "default" | "accent" | "warn" }) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.18em]",
        tone === "default" && "bg-stone-200 text-stone-700",
        tone === "accent" && "bg-sky-100 text-sky-900",
        tone === "warn" && "bg-amber-100 text-amber-900",
        className
      )}
      {...props}
    />
  );
}

export function Menubar({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "flex flex-wrap items-center gap-2 rounded-2xl border border-stone-300 bg-white/80 p-2 shadow-sm",
        className
      )}
      {...props}
    />
  );
}

export function Separator({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("h-px w-full bg-stone-200", className)} {...props} />;
}

export function TableShell({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("overflow-hidden rounded-[22px] border border-stone-300 bg-white/80", className)}
      {...props}
    />
  );
}
