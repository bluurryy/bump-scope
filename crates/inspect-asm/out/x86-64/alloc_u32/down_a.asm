inspect_asm::alloc_u32::down_a:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	add rax, -4
	cmp rax, qword ptr [rcx + 8]
	jb .LBB_2
	mov qword ptr [rcx], rax
	mov dword ptr [rax], esi
	pop rbx
	ret
.LBB_2:
	mov ebx, esi
	call bump_scope::bump_scope::BumpScope<A,_,_>::do_alloc_sized_in_another_chunk
	mov esi, ebx
	mov dword ptr [rax], esi
	pop rbx
	ret