import { useState, useEffect, useRef } from 'react';

function ExpandablePanel({ 
  isExpanded, 
  children, 
  className = '',
  animationDuration = 300 
}) {
  const [height, setHeight] = useState(0);
  const [shouldRender, setShouldRender] = useState(isExpanded);
  const contentRef = useRef(null);

  useEffect(() => {
    if (isExpanded) {
      setShouldRender(true);
    }

    const updateHeight = () => {
      if (contentRef.current) {
        const scrollHeight = contentRef.current.scrollHeight;
        setHeight(isExpanded ? scrollHeight : 0);
      }
    };

    // Update height on expansion state change
    updateHeight();

    // Handle collapse completion
    if (!isExpanded) {
      const timer = setTimeout(() => {
        setShouldRender(false);
      }, animationDuration);
      return () => clearTimeout(timer);
    }
  }, [isExpanded, animationDuration]);

  // Update height when content changes (e.g., data loads)
  useEffect(() => {
    if (isExpanded && contentRef.current) {
      const resizeObserver = new ResizeObserver(() => {
        if (contentRef.current) {
          setHeight(contentRef.current.scrollHeight);
        }
      });

      resizeObserver.observe(contentRef.current);
      return () => resizeObserver.disconnect();
    }
  }, [isExpanded, children]);

  return (
    <div
      className={`overflow-hidden transition-all duration-300 ease-in-out ${className}`}
      style={{ 
        height: `${height}px`,
        transitionDuration: `${animationDuration}ms`
      }}
    >
      <div ref={contentRef} className="w-full">
        {shouldRender && children}
      </div>
    </div>
  );
}

export default ExpandablePanel;