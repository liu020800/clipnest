import { useEffect, useRef, useState } from "react";

/**
 * 200px rootMargin 视口阈值懒加载。挂载时若已可见直接 true,
 * 不可见时挂 IntersectionObserver 监听, 进入 200px 视口时变 true。
 */
export function useInViewport<T extends Element = HTMLDivElement>(
  rootMargin = "200px",
): [React.RefObject<T | null>, boolean] {
  const ref = useRef<T | null>(null);
  const [inView, setInView] = useState(false);

  useEffect(() => {
    if (inView) return;
    const node = ref.current;
    if (!node) return;
    if (typeof IntersectionObserver === "undefined") {
      setInView(true);
      return;
    }
    const obs = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setInView(true);
            obs.disconnect();
            return;
          }
        }
      },
      { rootMargin, threshold: 0.01 },
    );
    obs.observe(node);
    return () => obs.disconnect();
  }, [inView, rootMargin]);

  return [ref, inView];
}
