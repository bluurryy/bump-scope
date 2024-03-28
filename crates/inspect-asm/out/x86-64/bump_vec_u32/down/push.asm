inspect_asm::bump_vec_u32::down::push:
	push rbp
	push rbx
	push rax
	mov rax, qword ptr [rdi + 8]
	cmp qword ptr [rdi + 16], rax
	je .LBB_1
.LBB_2:
	mov rcx, qword ptr [rdi]
	mov dword ptr [rcx + 4*rax], esi
	inc rax
	mov qword ptr [rdi + 8], rax
	add rsp, 8
	pop rbx
	pop rbp
	ret
.LBB_1:
	mov ebp, esi
	mov esi, 1
	mov rbx, rdi
	call bump_scope::mut_bump_vec::MutBumpVec<T,A,_,_>::generic_grow_cold
	mov esi, ebp
	mov rdi, rbx
	mov rax, qword ptr [rbx + 8]
	jmp .LBB_2